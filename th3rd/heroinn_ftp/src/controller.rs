use heroinn_util::{rpc::RpcMessage, ftp::DiskInfo};
use std::{io::*, sync::mpsc::Sender};

use crate::G_RPCCLIENT;


pub fn get_remote_disk_info(sender : &Sender<RpcMessage>) -> Result<Vec<DiskInfo>>{
    let msg = RpcMessage::build_call("get_disk_info" , vec![]);
    let mut remote_disk_info = vec![];
    sender.send(msg.clone()).unwrap();
    match G_RPCCLIENT.wait_msg(&msg.id, 10){
        Ok(p) => {
            for i in &p.data{
                let item = DiskInfo::parse(i).unwrap();
                remote_disk_info.push(item);
            }

            Ok(remote_disk_info)
        }
        Err(e) => {
            Err(e)
        }
    }
}