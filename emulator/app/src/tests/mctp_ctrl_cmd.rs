// Licensed under the Apache-2.0 license

use crate::i3c_socket::{MctpTestState, MctpTransportTest};
use crate::tests::mctp_util::base_protocol::{MCTPMsgHdr, MCTP_MSG_HDR_SIZE};
use crate::tests::mctp_util::common::MctpUtil;
use crate::tests::mctp_util::ctrl_protocol::*;
use crate::EMULATOR_RUNNING;
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use zerocopy::IntoBytes;

const TEST_TARGET_EID: u8 = 0xA;

type MCTPCtrlMsg = (
    MCTPMsgHdr<[u8; MCTP_MSG_HDR_SIZE]>,
    MCTPCtrlMsgHdr<[u8; MCTP_CTRL_MSG_HDR_SIZE]>,
    Vec<u8>,
);

const MCTP_MSG_HDR_OFFSET: usize = 0;
const MCTP_CTRL_MSG_HDR_OFFSET: usize = MCTP_MSG_HDR_OFFSET + MCTP_MSG_HDR_SIZE;
const MCTP_CTRL_PAYLOAD_OFFSET: usize = MCTP_CTRL_MSG_HDR_OFFSET + MCTP_CTRL_MSG_HDR_SIZE;

#[derive(EnumIter, Debug)]
pub(crate) enum MCTPCtrlCmdTests {
    SetEID,
    SetEIDForce,
    SetEIDNullFail,
    SetEIDBroadcastFail,
    SetEIDInvalidFail,
    GetEID,
}

impl MCTPCtrlCmdTests {
    pub fn generate_tests() -> Vec<Box<dyn MctpTransportTest + Send>> {
        MCTPCtrlCmdTests::iter()
            .enumerate()
            .map(|(i, test_id)| {
                let test_name = test_id.name();
                let req_msg = test_id.generate_request_msg();
                let resp_msg = test_id.generate_response_msg();
                let msg_tag = (i % 4) as u8;
                Box::new(Test::new(test_name, req_msg, resp_msg, msg_tag))
                    as Box<dyn MctpTransportTest + Send>
            })
            .collect()
    }

    fn generate_request_msg(&self) -> Vec<u8> {
        let mctp_common_msg_hdr = MCTPMsgHdr::new();

        let mut mctp_ctrl_msg_hdr = MCTPCtrlMsgHdr::new();
        mctp_ctrl_msg_hdr.set_rq(1);
        mctp_ctrl_msg_hdr.set_cmd(self.cmd());

        let req_data = match self {
            MCTPCtrlCmdTests::SetEID => set_eid_req_bytes(SetEIDOp::SetEID, TEST_TARGET_EID),
            MCTPCtrlCmdTests::SetEIDForce => {
                set_eid_req_bytes(SetEIDOp::ForceEID, TEST_TARGET_EID + 1)
            }
            MCTPCtrlCmdTests::SetEIDNullFail => set_eid_req_bytes(SetEIDOp::SetEID, 0),
            MCTPCtrlCmdTests::SetEIDBroadcastFail => set_eid_req_bytes(SetEIDOp::SetEID, 0xFF),
            MCTPCtrlCmdTests::SetEIDInvalidFail => set_eid_req_bytes(SetEIDOp::SetEID, 0x1),
            MCTPCtrlCmdTests::GetEID => {
                vec![]
            }
        };
        MCTPCtrlCmdTests::generate_msg((mctp_common_msg_hdr, mctp_ctrl_msg_hdr, req_data))
    }

    fn generate_response_msg(&self) -> Vec<u8> {
        let mctp_common_msg_hdr = MCTPMsgHdr::new();

        let mut mctp_ctrl_msg_hdr = MCTPCtrlMsgHdr::new();
        mctp_ctrl_msg_hdr.set_rq(0);
        mctp_ctrl_msg_hdr.set_cmd(self.cmd());

        let resp_data = match self {
            MCTPCtrlCmdTests::SetEID => set_eid_resp_bytes(
                CmdCompletionCode::Success,
                SetEIDStatus::Accepted,
                SetEIDAllocStatus::NoEIDPool,
                TEST_TARGET_EID,
            ),
            MCTPCtrlCmdTests::SetEIDForce => set_eid_resp_bytes(
                CmdCompletionCode::Success,
                SetEIDStatus::Accepted,
                SetEIDAllocStatus::NoEIDPool,
                TEST_TARGET_EID + 1,
            ),
            MCTPCtrlCmdTests::SetEIDNullFail => set_eid_resp_bytes(
                CmdCompletionCode::ErrorInvalidData,
                SetEIDStatus::Rejected,
                SetEIDAllocStatus::NoEIDPool,
                0,
            ),
            MCTPCtrlCmdTests::SetEIDBroadcastFail => set_eid_resp_bytes(
                CmdCompletionCode::ErrorInvalidData,
                SetEIDStatus::Rejected,
                SetEIDAllocStatus::NoEIDPool,
                0,
            ),
            MCTPCtrlCmdTests::SetEIDInvalidFail => set_eid_resp_bytes(
                CmdCompletionCode::ErrorInvalidData,
                SetEIDStatus::Rejected,
                SetEIDAllocStatus::NoEIDPool,
                0,
            ),
            MCTPCtrlCmdTests::GetEID => {
                get_eid_resp_bytes(CmdCompletionCode::Success, TEST_TARGET_EID + 1)
            }
        };

        MCTPCtrlCmdTests::generate_msg((mctp_common_msg_hdr, mctp_ctrl_msg_hdr, resp_data))
    }

    fn generate_msg(mctp_msg: MCTPCtrlMsg) -> Vec<u8> {
        let mut pkt: Vec<u8> = vec![0; MCTP_CTRL_PAYLOAD_OFFSET + mctp_msg.2.len()];

        mctp_msg
            .0
            .write_to(&mut pkt[MCTP_MSG_HDR_OFFSET..MCTP_MSG_HDR_OFFSET + MCTP_MSG_HDR_SIZE])
            .expect("mctp common msg header write failed");
        mctp_msg
            .1
            .write_to(
                &mut pkt
                    [MCTP_CTRL_MSG_HDR_OFFSET..MCTP_CTRL_MSG_HDR_OFFSET + MCTP_CTRL_MSG_HDR_SIZE],
            )
            .expect("mctp ctrl msg header write failed");
        pkt[MCTP_CTRL_PAYLOAD_OFFSET..].copy_from_slice(&mctp_msg.2);
        pkt
    }

    fn name(&self) -> &str {
        match self {
            MCTPCtrlCmdTests::SetEID => "SetEID",
            MCTPCtrlCmdTests::SetEIDForce => "SetEIDForce",
            MCTPCtrlCmdTests::SetEIDNullFail => "SetEIDNullFail",
            MCTPCtrlCmdTests::SetEIDBroadcastFail => "SetEIDBroadcastFail",
            MCTPCtrlCmdTests::SetEIDInvalidFail => "SetEIDInvalidFail",
            MCTPCtrlCmdTests::GetEID => "GetEID",
        }
    }

    fn cmd(&self) -> u8 {
        match self {
            MCTPCtrlCmdTests::SetEID
            | MCTPCtrlCmdTests::SetEIDForce
            | MCTPCtrlCmdTests::SetEIDNullFail
            | MCTPCtrlCmdTests::SetEIDBroadcastFail
            | MCTPCtrlCmdTests::SetEIDInvalidFail => MCTPCtrlCmd::SetEID as u8,
            MCTPCtrlCmdTests::GetEID => MCTPCtrlCmd::GetEID as u8,
        }
    }
}

#[derive(Debug, Clone)]
struct Test {
    name: String,
    test_state: MctpTestState,
    req_msg: Vec<u8>,
    resp_msg: Vec<u8>,
    msg_tag: u8,
    mctp_util: MctpUtil,
    passed: bool,
}

impl Test {
    fn new(name: &str, req_msg: Vec<u8>, resp_msg: Vec<u8>, msg_tag: u8) -> Self {
        Self {
            name: name.to_string(),
            test_state: MctpTestState::Start,
            req_msg,
            resp_msg,
            msg_tag,
            mctp_util: MctpUtil::new(),
            passed: false,
        }
    }

    fn check_response(&mut self, data: &[u8]) {
        if data.len() == self.resp_msg.len() && data == self.resp_msg {
            self.passed = true;
        }
    }

    fn pre_process(&mut self) {
        match self.name.as_str() {
            "SetEID" => {}
            _ => self.mctp_util.set_dest_eid(TEST_TARGET_EID),
        }
    }
}

impl MctpTransportTest for Test {
    fn is_passed(&self) -> bool {
        self.passed
    }

    fn run_test(&mut self, stream: &mut TcpStream, target_addr: u8) {
        stream.set_nonblocking(true).unwrap();
        while EMULATOR_RUNNING.load(Ordering::Relaxed) {
            match self.test_state {
                MctpTestState::Start => {
                    println!("Starting test: {}", self.name);
                    self.test_state = MctpTestState::SendReq;
                }
                MctpTestState::SendReq => {
                    self.pre_process();
                    self.mctp_util.send_request(
                        self.msg_tag,
                        self.req_msg.as_slice(),
                        stream,
                        target_addr,
                    );
                    self.test_state = MctpTestState::ReceiveResp;
                }
                MctpTestState::ReceiveResp => {
                    let resp_msg = self.mctp_util.receive_response(stream, target_addr, None);

                    if !resp_msg.is_empty() {
                        self.check_response(&resp_msg);
                        self.passed = true;
                    }
                    self.test_state = MctpTestState::Finish;
                }
                MctpTestState::Finish => {
                    println!(
                        "Test {} : {}",
                        self.name,
                        if self.passed { "PASSED" } else { "FAILED" }
                    );
                    break;
                }
                _ => {}
            }
        }
    }
}
