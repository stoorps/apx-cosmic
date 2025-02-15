use std::collections::HashMap;

use anyhow::Result;
use serde::Deserialize;
use tracing::{info, warn, error};
use udisks2::{block::BlockProxy, drive::DriveProxy, partition::PartitionProxy, partitiontable::PartitionTableProxy, Client};
use zbus::{
    zvariant::{self, OwnedObjectPath, Value},
    Connection,
};
use zbus_macros::proxy;

use super::{get_usage_data, manager::UDisks2ManagerProxy, PartitionModel};

#[derive(Debug, Clone)]
pub struct DriveModel {
    pub can_power_off: bool,
    pub ejectable: bool,
    pub media_available: bool,
    pub media_change_detected: bool,
    pub media_removable: bool,
    pub optical: bool,
    pub optical_blank: bool,
    pub removable: bool,
    pub id: String,
    pub model: String,
    pub revision: String,
    pub serial: String,
    pub vendor: String,
    pub size: u64,
    pub name: String,
    pub block_path: String,
    pub partitions: Vec<PartitionModel>,
    pub path: String,
}


#[derive(Debug, Clone)]
struct DriveBlockPair {
    block_path: OwnedObjectPath,
    drive_path: OwnedObjectPath,
}

impl DriveModel {
    pub fn pretty_name(&self) -> String {
        self.name.split("/").last().unwrap().replace("_", " ") //TODO: Handle unwrap
    }

    pub(crate) async fn from_proxy(
        path: &str,
        drive_proxy: &DriveProxy<'_>,
    ) -> Result<Self> {
        Ok(DriveModel {
            name: path.to_owned(),
            path: path.to_string(),
            size: drive_proxy.size().await?,
            id: drive_proxy.id().await?,
            model: drive_proxy.model().await?,
            serial: drive_proxy.serial().await?,
            vendor: drive_proxy.vendor().await?,
            block_path: path.to_string(),
            partitions: vec![],
            can_power_off: drive_proxy.can_power_off().await?,
            ejectable: drive_proxy.ejectable().await?,
            media_available: drive_proxy.media_available().await?,
            media_change_detected: drive_proxy.media_change_detected().await?,
            media_removable: drive_proxy.media_removable().await?,
            optical: drive_proxy.optical().await?,
            optical_blank: drive_proxy.optical_blank().await?,
            removable: drive_proxy.removable().await?,
            revision: drive_proxy.revision().await?,
        })
    }

    async fn get_drive_paths(connection: &Connection) -> Result<Vec<DriveBlockPair>> {
        let manager_proxy = UDisks2ManagerProxy::new(&connection).await?;
        let block_paths = manager_proxy.get_block_devices(HashMap::new()).await?;

        let mut drive_paths: Vec<DriveBlockPair> = vec![];

        for path in block_paths {
            let block_device = match BlockProxy::builder(&connection).path(&path)?.build().await {
                Ok(d) => d,
                Err(e) => {
                    info!("Could not get block device: {}", e);
                    continue;
                }
            };

            //Drive nodes don't have a .Partition interface assigned.
            let _ = match PartitionProxy::builder(&connection).path(&path)?.build().await {
                Ok(e) => match e.table().await {
                    Ok(_) => {
                        continue;
                    }
                    Err(_) => { } //We've found a drive
                },
                Err(_) => { } //We've found a drive
            };

            match block_device.drive().await {
                Ok(dp) => drive_paths.push(DriveBlockPair {
                    block_path: path,
                    drive_path: dp,
                }),
                Err(_) => continue,
            }
        }

        Ok(drive_paths)
    }

    pub async fn get_drives() -> Result<Vec<DriveModel>> {
        let connection = Connection::system().await?;
        let client = Client::new_for_connection(Connection::system().await?).await?;
        let drive_paths = Self::get_drive_paths(&connection).await?;


        let mut drives: HashMap<String, DriveModel> = HashMap::new();
        let mut usage_data = get_usage_data()?;

        for pair in drive_paths {
            let drive_proxy = DriveProxy::builder(&connection).path(&pair.drive_path)?.build().await?;
            let mut drive = match DriveModel::from_proxy(&pair.drive_path, &drive_proxy).await {
                Ok(d) => d,
                Err(e) => {
                    warn!("Could not get drive: {}", e);
                    continue;
                }
            };

            let partition_table_proxy =
                match PartitionTableProxy::builder(&connection).path(&pair.block_path)?.build().await {
                    Ok(p) => p,
                    Err(e) => {
                        error!("Error getting partition table: {}", e);
                        continue;
                    }
                };

            let partition_paths = match partition_table_proxy.partitions().await {
                Ok(p) => p,
                Err(e) => {
                    error!("Error getting partitions for {}: {}", pair.block_path, e);
                    continue;
                }
            };

            for partition_path in partition_paths {
                let partition_proxy =
                    match PartitionProxy::builder(&connection).path(&partition_path)?.build().await {
                        Ok(p) => p,
                        Err(e) => {
                            error!("Error getting partition info: {}", e);
                            continue;
                        }
                    };

                let short_name = partition_path.as_str().split("/").last();

                let usage = match short_name {
                    Some(sn) => match usage_data.iter_mut().find(|u| u.filesystem.ends_with(sn)) {
                        Some(u) => Some(u.clone()),
                        None => None,
                    },
                    None => None,
                };


                drive.partitions.push(PartitionModel::from_proxy(&client, partition_path.clone(), usage, partition_proxy).await?);
            }


            drives.insert(drive.name.clone(), drive);
        }

        //Order b
        let mut drives: Vec<DriveModel> = drives.into_values().collect();
        drives.sort_by(|d1, d2| {
            d1.removable.cmp(&d2.removable).then_with(|| {
                d2.block_path.cmp(&d1.block_path) //TODO: understand this. d1 SHOULD come first in this compare... 
            })
        });
        

        Ok(drives)
    }
}


