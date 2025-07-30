// Licensed under the Apache-2.0 license

use crate::doe_mbox_fsm::{DoeTestState, DoeTransportTest};
use crate::{sleep_emulator_ticks, EMULATOR_RUNNING};
use rand::Rng;
const NUM_TEST_VECTORS: usize = 10;
const MIN_TEST_DATA_DWORDS: usize = 1; // minimum size of test vectors
const MAX_TEST_DATA_DWORDS: usize = 250; // maximum size of test vectors
use crate::tests::doe_util::common::DoeUtil;
use crate::tests::doe_util::protocol::DataObjectType;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};

struct Test {
    test_vector: Vec<u8>,
    test_state: DoeTestState,
    passed: bool,
    retry_count: usize,
}

pub fn generate_tests() -> Vec<Box<dyn DoeTransportTest + Send>> {
    let mut rng = rand::thread_rng();
    let mut tests: Vec<Box<dyn DoeTransportTest + Send>> = Vec::new();
    for _ in 0..NUM_TEST_VECTORS {
        // Generate a random size (multiple of 4 bytes)
        let num_words = rng.gen_range((MIN_TEST_DATA_DWORDS)..=(MAX_TEST_DATA_DWORDS));
        let mut vector = vec![0u8; num_words * 4];
        rng.fill(vector.as_mut_slice());
        tests.push(Box::new(Test {
            test_vector: vector,
            test_state: DoeTestState::Start,
            passed: false,
            retry_count: 40,
        }));
    }
    tests
}

impl DoeTransportTest for Test {
    fn run_test(
        &mut self,
        tx: &mut Sender<Vec<u8>>,
        rx: &mut Receiver<Vec<u8>>,
        wait_for_responder: bool,
    ) {
        println!(
            "DOE_USER_LOOPBACK: Running test with test vector len {}",
            self.test_vector.len()
        );

        self.test_state = DoeTestState::Start;

        while EMULATOR_RUNNING.load(Ordering::Relaxed) {
            match self.test_state {
                DoeTestState::Start => {
                    if wait_for_responder {
                        sleep_emulator_ticks(1_000_000);
                    }
                    self.test_state = DoeTestState::SendData;
                }
                DoeTestState::SendData => {
                    if DoeUtil::send_data_object(&self.test_vector, DataObjectType::DoeSpdm, tx)
                        .is_ok()
                    {
                        self.test_state = DoeTestState::ReceiveData;
                        sleep_emulator_ticks(100_000);
                    } else {
                        println!("DOE_USER_LOOPBACK: Failed to send request");
                        self.passed = false;
                        self.test_state = DoeTestState::Finish;
                    }
                }
                DoeTestState::ReceiveData => match DoeUtil::receive_data_object(rx) {
                    Ok(response) if !response.is_empty() => {
                        if response == self.test_vector {
                            println!(
                                "DOE_USER_LOOPBACK: Received response matches expected with len {}",
                                response.len()
                            );
                            self.passed = true;
                        } else {
                            println!(
                                "DOE_USER_LOOPBACK: Received response does not match expected: {:?} != {:?}",
                                response, self.test_vector
                            );
                            self.passed = false;
                        }
                        self.test_state = DoeTestState::Finish;
                    }
                    Ok(_) => {
                        if self.retry_count > 0 {
                            self.retry_count -= 1;
                            // Stay in ReceiveData state and yield for a bit
                            std::thread::sleep(std::time::Duration::from_millis(300));
                            println!(
                                "DOE_USER_LOOPBACK: No response received, retrying... ({} retries left)",
                                self.retry_count
                            );
                        } else {
                            println!("DOE_USER_LOOPBACK: No response received after retries, failing test");
                            self.passed = false;
                            self.test_state = DoeTestState::Finish;
                        }
                    }
                    Err(e) => {
                        println!("DOE_USER_LOOPBACK: Failed to receive response: {:?}", e);
                        self.passed = false;
                        self.test_state = DoeTestState::Finish;
                    }
                },
                DoeTestState::Finish => {
                    println!(
                        "DOE_DISCOVERY_TEST: Test with data len {} {}",
                        self.test_vector.len(),
                        if self.passed { "passed!" } else { "failed!" }
                    );
                    break;
                }
            }
        }
    }

    fn is_passed(&self) -> bool {
        self.passed
    }
}
