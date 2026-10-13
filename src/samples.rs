//! Sample logs for a local stack, so the pages have something to show:
//! the three gamelogs of the parser fixtures under fixed uuids, stored
//! the way an upload stores them (names resolved through the configured
//! ESI). Seeding again skips the logs that exist.

use uuid::Uuid;

use crate::server::AppState;

pub struct Sample {
    pub uuid: Uuid,
    pub file_name: &'static str,
    pub contents: &'static [u8],
}

pub const SAMPLES: [Sample; 3] = [
    Sample {
        uuid: Uuid::from_u128(0x5a3b_6e1c_0d4f_4a2b_8c7d_1e2f_3a4b_5c01),
        file_name: "20260212_094144_testlog.txt",
        contents: include_bytes!("../tests/fixtures/parser/testlog.txt"),
    },
    Sample {
        uuid: Uuid::from_u128(0x5a3b_6e1c_0d4f_4a2b_8c7d_1e2f_3a4b_5c02),
        file_name: "20260212_110210_neuts.txt",
        contents: include_bytes!("../tests/fixtures/parser/testlog2.txt"),
    },
    Sample {
        uuid: Uuid::from_u128(0x5a3b_6e1c_0d4f_4a2b_8c7d_1e2f_3a4b_5c03),
        file_name: "20260528_182653_fleet.txt",
        contents: include_bytes!("../tests/fixtures/parser/testlog3.txt"),
    },
];

/// Stores the samples that are missing; returns how many were added.
pub async fn seed(_state: &AppState) -> Result<usize, std::io::Error> {
    unimplemented!("not ported yet")
}
