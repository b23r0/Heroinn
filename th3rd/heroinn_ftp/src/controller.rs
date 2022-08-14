use heroinn_util::{rpc::RpcMessage, ftp::{method::{get_disk_info, get_folder_info}, FileInfo}};
use std::{io::*, sync::mpsc::Sender};

use crate::G_RPCCLIENT;


pub fn get_remote_disk_info(sender : &Sender<RpcMessage>) -> Result<Vec<FileInfo>>{
    let msg = RpcMessage::build_call("get_disk_info" , vec![]);
    let mut remote_disk_info = vec![];
    sender.send(msg.clone()).unwrap();
    match G_RPCCLIENT.wait_msg(&msg.id, 10){
        Ok(p) => {
            for i in &p.data{
                let item = FileInfo::parse(i).unwrap();
                remote_disk_info.push(item);
            }

            Ok(remote_disk_info)
        }
        Err(e) => {
            Err(e)
        }
    }
}

pub fn get_local_disk_info() -> Result<Vec<FileInfo>>{
    let mut local_disk_info = vec![];
    match get_disk_info(vec![]){
        Ok(p) => {
            for i in &p{
                let item = FileInfo::parse(i).unwrap();
                local_disk_info.push(item);
            }

            Ok(local_disk_info)
        }
        Err(e) => {
            Err(e)
        }
    }
}

pub fn get_remote_folder_info(sender : &Sender<RpcMessage> , full_path : &String) -> Result<Vec<FileInfo>>{
    let msg = RpcMessage::build_call("get_folder_info" , vec![full_path.clone()]);
    let mut remote_folder_info = vec![];
    sender.send(msg.clone()).unwrap();
    match G_RPCCLIENT.wait_msg(&msg.id, 10){
        Ok(p) => {
            for i in &p.data{
                let item = FileInfo::parse(i).unwrap();
                remote_folder_info.push(item);
            }

            Ok(remote_folder_info)
        }
        Err(e) => {
            Err(e)
        }
    }
}

pub fn get_remote_join_path(sender : &Sender<RpcMessage> , cur_path : &String , filename : &String) -> Result<String>{
    let msg = RpcMessage::build_call("join_path" , vec![cur_path.clone() , filename.clone()]);
    sender.send(msg.clone()).unwrap();
    match G_RPCCLIENT.wait_msg(&msg.id, 10){
        Ok(p) => {
            Ok(p.data[0].clone())
        }
        Err(e) => {
            Err(e)
        }
    }
}

pub fn get_local_folder_info(full_path : &String) -> Result<Vec<FileInfo>>{
    let mut local_folder_info = vec![];
    match get_folder_info(vec![full_path.clone()]){
        Ok(p) => {
            for i in &p{
                let item = FileInfo::parse(i).unwrap();
                local_folder_info.push(item);
            }

            Ok(local_folder_info)
        }
        Err(e) => {
            Err(e)
        }
    }
}