use landscape_common::database::{LandscapeDBTrait, LandscapeServiceDBTrait};
use landscape_common::observer::IfaceObserverAction;
use landscape_common::service::controller_service_v2::ControllerService;
use landscape_common::service::service_manager_v2::ServiceManager;
use landscape_common::{
    config::nat::{NatConfig, NatServiceConfig},
    service::{
        service_manager_v2::ServiceStarterTrait, DefaultServiceStatus, DefaultWatchServiceStatus,
        ServiceStatus,
    },
};
use landscape_database::nat::repository::NatServiceRepository;
use landscape_database::provider::LandscapeDBServiceProvider;
use tokio::sync::{broadcast, oneshot};

use crate::iface::get_iface_by_name;

#[derive(Clone, Default)]
pub struct NatService;

#[async_trait::async_trait]
impl ServiceStarterTrait for NatService {
    type Status = DefaultServiceStatus;
    type Config = NatServiceConfig;

    async fn start(&self, config: NatServiceConfig) -> DefaultWatchServiceStatus {
        let service_status = DefaultWatchServiceStatus::new();
        // service_status.just_change_status(ServiceStatus::Staring);

        if config.enable {
            // 具体的 NAT 服务启动逻辑
            if let Some(iface) = get_iface_by_name(&config.iface_name).await {
                let status_clone = service_status.clone();
                tokio::spawn(async move {
                    create_nat_service(
                        iface.index as i32,
                        iface.mac.is_some(),
                        config.nat_config,
                        status_clone,
                    )
                    .await
                });
            } else {
                tracing::error!("Interface {} not found", config.iface_name);
            }
        }

        service_status
    }
}

pub async fn create_nat_service(
    ifindex: i32,
    has_mac: bool,
    nat_config: NatConfig,
    service_status: DefaultWatchServiceStatus,
) {
    service_status.just_change_status(ServiceStatus::Staring);
    let (tx, rx) = oneshot::channel::<()>();
    let (other_tx, other_rx) = oneshot::channel::<()>();
    service_status.just_change_status(ServiceStatus::Running);
    let service_status_clone = service_status.clone();
    tokio::spawn(async move {
        let stop_wait = service_status_clone.wait_to_stopping();
        tracing::info!("等待外部停止信号");
        let _ = stop_wait.await;
        tracing::info!("接收外部停止信号");
        let _ = tx.send(());
        tracing::info!("向内部发送停止信号");
    });
    std::thread::spawn(move || {
        landscape_ebpf::nat::init_nat(ifindex, has_mac, rx, nat_config);
        tracing::info!("向外部线程发送解除阻塞信号");
        let _ = other_tx.send(());
    });
    let _ = other_rx.await;
    tracing::info!("结束外部线程阻塞");
    service_status.just_change_status(ServiceStatus::Stop);
}

#[derive(Clone)]
pub struct NatServiceManagerService {
    store: NatServiceRepository,
    service: ServiceManager<NatService>,
}

impl ControllerService for NatServiceManagerService {
    type Id = String;
    type Config = NatServiceConfig;
    type DatabseAction = NatServiceRepository;
    type H = NatService;

    fn get_service(&self) -> &ServiceManager<Self::H> {
        &self.service
    }

    fn get_repository(&self) -> &Self::DatabseAction {
        &self.store
    }
}

impl NatServiceManagerService {
    pub async fn new(
        store_service: LandscapeDBServiceProvider,
        mut dev_observer: broadcast::Receiver<IfaceObserverAction>,
    ) -> Self {
        let store = store_service.nat_service_store();
        let service = ServiceManager::init(store.list().await.unwrap(), Default::default()).await;

        let service_clone = service.clone();
        tokio::spawn(async move {
            while let Ok(msg) = dev_observer.recv().await {
                match msg {
                    IfaceObserverAction::Up(iface_name) => {
                        tracing::info!("restart {iface_name} Nat service");
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

        let store = store_service.nat_service_store();
        Self { service, store }
    }
}
