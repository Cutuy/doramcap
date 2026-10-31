//! Generated package hierarchy shared by planner contracts and execution APIs.
pub(crate) mod doramcap {
    pub mod compose {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/doramcap.compose.v1.rs"));
        }
        pub mod planner {
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/doramcap.compose.planner.v1.rs"));
            }
        }
    }
    pub mod dora {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/doramcap.dora.v1.rs"));
        }
    }
}
