use std::net::IpAddr;

#[derive(Debug, Clone, Copy)]
pub struct Speaking {
    pub user_id: Option<UserId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(pub u64);

#[derive(Debug, Clone)]
pub struct ProtocolData {
    pub address: IpAddr,
}

pub mod id {
    pub use super::UserId;
}
