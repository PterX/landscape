use std::net::IpAddr;

use landscape_common::database::{LandscapeDBTrait, LandscapeServiceDBTrait};
use landscape_common::route::{LanRouteInfo, RouteTargetInfo};
use landscape_common::LANDSCAPE_DEFAULE_DHCP_V4_CLIENT_PORT;
use landscape_common::{
    args::LAND_HOSTNAME,
    config::iface_ip::{IfaceIpModelConfig, IfaceIpServiceConfig},
    global_const::default_router::{RouteInfo, RouteType, LD_ALL_ROUTERS},
    observer::IfaceObserverAction,
    service::{
        controller_service_v2::ControllerService,
        service_manager_v2::{ServiceManager, ServiceStarterTrait},
        DefaultServiceStatus, DefaultWatchServiceStatus, ServiceStatus,
    },
};
use landscape_database::{
    iface_ip::repository::IfaceIpServiceRepository, provider::LandscapeDBServiceProvider,
};
use tokio::sync::broadcast;

use crate::route::IpRouteService;
use crate::{dev::LandscapeInterface, iface::get_iface_by_name};

#[derive(Clone)]
#[allow(dead_code)]
pub struct IPConfigService {
    route_service: IpRouteService,
}

impl IPConfigService {
    pub fn new(route_service: IpRouteService) -> Self {
        IPConfigService { route_service }
    }
}
#[async_trait::async_trait]
impl ServiceStarterTrait for IPConfigService {
    type Status = DefaultServiceStatus;

    type Config = IfaceIpServiceConfig;

    async fn start(&self, config: IfaceIpServiceConfig) -> DefaultWatchServiceStatus {
        let service_status = DefaultWatchServiceStatus::new();

        if config.enable {
            if let Some(iface) = get_iface_by_name(&config.iface_name).await {
                let status_clone = service_status.clone();

                let route_service = self.route_service.clone();
                tokio::spawn(async move {
                    init_service_from_config(iface, config.ip_model, status_clone, route_service)
                        .await
                });
            } else {
                tracing::error!("Interface {} not found", config.iface_name);
            }
        }

        service_status
    }
}

async fn init_service_from_config(
    iface: LandscapeInterface,
    service_config: IfaceIpModelConfig,
    service_status: DefaultWatchServiceStatus,
    route_service: IpRouteService,
) {
    match service_config {
        IfaceIpModelConfig::Nothing => {}
        IfaceIpModelConfig::Static {
            default_router, default_router_ip, ipv4, ipv4_mask, ..
        } => {
            // TODO: IPV6 的设置
            if let Some(ipv4) = ipv4 {
                service_status.just_change_status(ServiceStatus::Staring);
                let iface_name = iface.name;
                tracing::info!("set ipv4 is: {}", ipv4);
                let _ = std::process::Command::new("ip")
                    .args(&["addr", "add", &format!("{}/{}", ipv4, ipv4_mask), "dev", &iface_name])
                    .output();
                tracing::debug!("start setting");
                landscape_ebpf::map_setting::add_wan_ip(iface.index, ipv4.clone());

                let lan_info = LanRouteInfo {
                    ifindex: iface.index,
                    iface_name: iface_name.clone(),
                    iface_ip: IpAddr::V4(ipv4),
                    mac: iface.mac,
                    prefix: ipv4_mask,
                };
                route_service.insert_ipv4_lan_route(&iface_name, lan_info).await;

                if let Some(default_router_ip) = default_router_ip {
                    if !default_router_ip.is_broadcast()
                        && !default_router_ip.is_unspecified()
                        && !default_router_ip.is_loopback()
                    {
                        if default_router {
                            tracing::info!("setting default route: {:?}", default_router_ip);
                            LD_ALL_ROUTERS
                                .add_route(RouteInfo {
                                    iface_name: iface_name.clone(),
                                    weight: 1,
                                    route: RouteType::Ipv4(default_router_ip.clone()),
                                })
                                .await;
                        } else {
                            LD_ALL_ROUTERS.del_route_by_iface(&iface_name).await;
                        }

                        let info = RouteTargetInfo {
                            ifindex: iface.index,
                            weight: 1,
                            has_mac: iface.mac.is_some(),
                            is_docker: false,
                            iface_name: iface_name.clone(),
                            iface_ip: IpAddr::V4(ipv4),
                            default_route: default_router,
                            gateway_ip: IpAddr::V4(default_router_ip),
                        };
                        route_service.insert_ipv4_wan_route(&iface_name, info).await;
                    }
                }

                service_status.just_change_status(ServiceStatus::Running);
                service_status.wait_to_stopping().await;
                let _ = std::process::Command::new("ip")
                    .args(&["addr", "del", &format!("{}/{}", ipv4, ipv4_mask), "dev", &iface_name])
                    .output();

                if default_router {
                    LD_ALL_ROUTERS.del_route_by_iface(&iface_name).await;
                }
                route_service.remove_ipv4_wan_route(&iface_name).await;
                route_service.remove_ipv4_lan_route(&iface_name).await;
                landscape_ebpf::map_setting::del_wan_ip(iface.index);
                service_status.just_change_status(ServiceStatus::Stop);
            }
        }
        IfaceIpModelConfig::PPPoE { username: _, password: _, mtu: _, .. } => {
            // TODO： 重构 PPPoE ebpf 版本
            // if let Some(mac_addr) = iface.mac {
            //     let iface_name = iface.name.clone();
            //     let service_status = ip_config.clone();
            //     crate::pppoe_client::pppoe_client_v2::create_pppoe_client(
            //         iface.index,
            //         iface_name,
            //         mac_addr,
            //         username,
            //         password,
            //         service_status,
            //     )
            //     .await;
            // } else {
            //     ip_config.send_replace(ServiceStatus::Stop {
            //         message: Some("mac addr is empty".into()),
            //     });
            // }
        }
        IfaceIpModelConfig::DhcpClient { default_router, hostname, custome_opts: _ } => {
            if let Some(mac_addr) = iface.mac {
                let hostname =
                    hostname.filter(|h| !h.is_empty()).unwrap_or_else(|| LAND_HOSTNAME.clone());
                crate::dhcp_client::v4::dhcp_v4_client(
                    iface.index,
                    iface.name,
                    mac_addr,
                    LANDSCAPE_DEFAULE_DHCP_V4_CLIENT_PORT,
                    service_status,
                    hostname,
                    default_router,
                    route_service,
                )
                .await;
            } else {
                service_status.just_change_status(ServiceStatus::Stop);
            }
        }
    };
}

#[derive(Clone)]
pub struct IfaceIpServiceManagerService {
    store: IfaceIpServiceRepository,
    service: ServiceManager<IPConfigService>,
}

impl ControllerService for IfaceIpServiceManagerService {
    type Id = String;
    type Config = IfaceIpServiceConfig;
    type DatabseAction = IfaceIpServiceRepository;
    type H = IPConfigService;

    fn get_service(&self) -> &ServiceManager<Self::H> {
        &self.service
    }

    fn get_repository(&self) -> &Self::DatabseAction {
        &self.store
    }
}

impl IfaceIpServiceManagerService {
    pub async fn new(
        route_service: IpRouteService,
        store_service: LandscapeDBServiceProvider,
        mut dev_observer: broadcast::Receiver<IfaceObserverAction>,
    ) -> Self {
        let store = store_service.iface_ip_service_store();
        let server_starter = IPConfigService::new(route_service);
        let service =
            ServiceManager::init(store.list().await.unwrap(), server_starter.clone()).await;

        let service_clone = service.clone();
        tokio::spawn(async move {
            while let Ok(msg) = dev_observer.recv().await {
                match msg {
                    IfaceObserverAction::Up(iface_name) => {
                        tracing::info!("restart {iface_name} IfaceIp service");
                        let service_config = if let Some(service_config) =
                            store.find_by_iface_name(iface_name.clone()).await.unwrap()
                        {
                            service_config
                        } else {
                            continue;
                        };

                        let _ = service_clone.update_service(service_config).await;
                    }
                    IfaceObserverAction::Down(_) => {}
                }
            }
        });

        let store = store_service.iface_ip_service_store();
        Self { service, store }
    }
}
