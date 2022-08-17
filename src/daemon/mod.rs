use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::time::SystemTime;

use crate::class::{EptFileNode, LazyDeleteNode};
use crate::hash_service::HashService;
use crate::scanner::Scanner;

pub struct Daemon {
    timestamp_recent_finish: SystemTime,   //上次扫描结束时的时间戳
    status_running: bool,                  //是否有一个扫描任务正在进行中
    list_lazy_delete: Vec<LazyDeleteNode>, //懒删除文件列表

    commander: Receiver<String>, //更新请求接收器
    result_sender: Sender<HashMap<String, Vec<EptFileNode>>>, //结果发送channel
    scanner: Scanner,            //扫描器实例
    dir_packages: String,        //插件包所在目录
}
impl Daemon {
    pub fn new(
        commander: Receiver<String>,
        result_sender: Sender<HashMap<String, Vec<EptFileNode>>>,
        hash_map: HashMap<String, String>,
        dir_packages: String,
    ) -> Self {
        let hash_service = HashService::new(hash_map);
        let scanner = Scanner::new(hash_service);
        Daemon {
            timestamp_recent_finish: SystemTime::UNIX_EPOCH,
            status_running: false,
            list_lazy_delete: vec![],
            result_sender,
            dir_packages,
            scanner,
            commander,
        }
    }

    pub fn serve(&mut self) {
        let cmd_request = String::from("request");
        while let Ok(cmd)=self.commander.recv() {
            println!("Daemon Info:Get cmd : {}", &cmd);
            if cmd == cmd_request {
                self.request();
            }
        }
    }

    //由外部调用，要求安排一次扫描更新
    pub fn request(&mut self) {
        //判断是否使能扫描
        if !self.status_running
            && SystemTime::now()
                .duration_since(self.timestamp_recent_finish)
                .unwrap()
                .as_secs()
                > 5 * 60
        {
            self.status_running = true;
            let update_res = self.update();
            if let Err(err) = update_res {
                println!("Error:Can't update packages : {:?}", err);
            }
            self.timestamp_recent_finish = SystemTime::now();
            self.status_running = false;
        }
    }

    //执行一次更新
    fn update(&mut self) -> std::io::Result<()> {
        println!("Info:Start updating");

        //懒删除
        for node in &self.list_lazy_delete {
            self.scanner
                .delete_file(node.path.to_owned(), node.key.to_owned())
        }

        //生成新的扫描结果和懒删除列表
        let (result, lazy_delete_list) = self.scanner.scan_packages(self.dir_packages.clone())?;

        //发送结果
        self.result_sender.send(result);

        //更新懒删除列表
        self.list_lazy_delete = lazy_delete_list;

        println!("Info:Finish updating");
        Ok(())
    }
}
