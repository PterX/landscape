use std::{collections::VecDeque, net::Ipv4Addr};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{net::MacAddr, LAND_ARP_INFO_SIZE};

#[derive(Debug, Serialize, Deserialize, Clone, Default, TS)]
#[ts(export, export_to = "common/dhcp_v4_server.d.ts")]
pub struct DHCPv4OfferInfo {
    pub boot_time: f64,
    #[ts(type = "number")]
    pub relative_boot_time: u64,
    pub offered_ips: Vec<DHCPv4OfferInfoItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export, export_to = "common/dhcp_v4_server.d.ts")]
pub struct DHCPv4OfferInfoItem {
    pub hostname: Option<String>,
    pub mac: MacAddr,
    pub ip: Ipv4Addr,
    #[ts(type = "number")]
    pub relative_active_time: u64,
    pub expire_time: u32,
    pub is_static: bool,
}

pub struct ArpScanStatus {
    infos: VecDeque<ArpScanInfo>,
}

impl ArpScanStatus {
    pub fn new() -> Self {
        Self { infos: VecDeque::with_capacity(LAND_ARP_INFO_SIZE) }
    }

    pub fn insert_new_info(&mut self, value: ArpScanInfo) {
        if self.infos.len() == LAND_ARP_INFO_SIZE {
            self.infos.pop_front();
        }

        self.infos.push_back(value);
    }

    pub fn get_arp_info(&self) -> Vec<ArpScanInfo> {
        self.infos.iter().cloned().collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export, export_to = "common/dhcp_v4_server.d.ts")]
pub struct ArpScanInfo {
    infos: Vec<ArpScanInfoItem>,
}

impl ArpScanInfo {
    pub fn new(infos: Vec<ArpScanInfoItem>) -> Self {
        Self { infos }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export, export_to = "common/dhcp_v4_server.d.ts")]
pub struct ArpScanInfoItem {
    pub ip: Ipv4Addr,
    pub mac: MacAddr,
}
