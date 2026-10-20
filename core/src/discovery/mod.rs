use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use thiserror::Error;

const SERVICE_TYPE: &str = "_photon._tcp.local.";

#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("mDNS error: {0}")]
    MdnsError(#[from] mdns_sd::Error),
}

#[derive(Clone, Debug)]
pub struct Device {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub pubkey_fingerprint: String,
}

pub struct Discovery {
    daemon: ServiceDaemon,
    discovered_devices: Arc<Mutex<HashMap<String, Device>>>,
}

impl Discovery {
    pub fn new() -> Result<Self, DiscoveryError> {
        let daemon = ServiceDaemon::new()?;
        Ok(Self {
            daemon,
            discovered_devices: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn start_broadcasting(
        &self,
        instance_name: &str,
        port: u16,
        pubkey_fingerprint: &str,
    ) -> Result<(), DiscoveryError> {
        let properties = [("fingerprint", pubkey_fingerprint)];
        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            instance_name,
            &format!("{}.local.", instance_name),
            "",
            port,
            &properties[..],
        )?;

        self.daemon.register(service_info)?;
        Ok(())
    }

    pub fn stop_broadcasting(&self, instance_name: &str) -> Result<(), DiscoveryError> {
        let fullname = format!("{}.{}", instance_name, SERVICE_TYPE);
        self.daemon.unregister(&fullname)?;
        Ok(())
    }

    pub async fn start_listening(
        &self,
        sender: mpsc::Sender<Device>,
    ) -> Result<(), DiscoveryError> {
        let receiver = self.daemon.browse(SERVICE_TYPE)?;
        let devices = self.discovered_devices.clone();

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let fingerprint = info
                            .get_property("fingerprint")
                            .map(|p| p.val_str().to_string())
                            .unwrap_or_default();

                        let ip = info.get_addresses().iter().next().map(|ip| ip.to_string());

                        if let Some(ip) = ip {
                            let device = Device {
                                name: info.get_fullname().to_string(),
                                ip,
                                port: info.get_port(),
                                pubkey_fingerprint: fingerprint,
                            };

                            devices.lock().unwrap().insert(info.get_fullname().to_string(), device.clone());
                            let _ = sender.send(device).await;
                        }
                    }
                    ServiceEvent::ServiceRemoved(_service_type, fullname) => {
                        devices.lock().unwrap().remove(&fullname);
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }
}
