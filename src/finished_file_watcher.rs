use std::{collections::HashMap, ops::Range, path::{Path, PathBuf}, sync::mpsc::{Receiver, SendError, Sender, channel}, thread, time::{self, Duration, SystemTime}};

use notify::*;

pub fn finishedFiles(directory:PathBuf,last_write_delay:Duration,output_tx:Sender<PathBuf>){
    let (file_writen_tx, file_written_rx) = channel();
    
    thread::spawn(move ||{waitForFiles(last_write_delay,  file_written_rx,output_tx)});
    thread::spawn(move ||{
        watch_dir_writes(directory,Duration::from_secs(5), file_writen_tx)
        });
}
fn print_send_err<T>(err:SendError<T>){println!("Error whilst sending watched event: {:?}",err);}
///Watches a directory and transmits the path and time of write of any new files being written to 
///for information on debounce_delay see "notify::watcher"
fn watch_dir_writes(directory:PathBuf,debounce_delay:Duration,out_tx:Sender<(PathBuf,SystemTime)>){
    let get_write_events=|event:DebouncedEvent|{
        match event{
            DebouncedEvent::Create(buf)=>{
                out_tx.send((buf,time::SystemTime::now())).map_err(print_send_err);
            },
            DebouncedEvent::Write(buf)=>{
                out_tx.send((buf,time::SystemTime::now())).map_err(print_send_err);
            },
            _=>()
        }
        
    };
    let (wtx, wrx) = channel();
        
    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = watcher(wtx, debounce_delay).unwrap();
    
    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(directory, RecursiveMode::NonRecursive).unwrap();
    wrx.iter().for_each(get_write_events);
}
///
///Transmits a file when it has not received an event about that file for the delay time
fn waitForFiles(delay:time::Duration,file_written_rx:Receiver<(PathBuf,time::SystemTime)>, tx_files_finished_writing:Sender<PathBuf>){
    let mut files:HashMap<PathBuf,SystemTime>=HashMap::new();
    loop{
        file_written_rx.try_iter().for_each(|(path,time)|{files.insert(path, time); });

        let now=time::SystemTime::now();
        let mut  toremove:Vec<_>=files.iter().filter_map(|(path,last_write_time )|{
            if *last_write_time+delay>= now {
                tx_files_finished_writing.send(path.clone()).map_err(print_send_err);
                Some(path.clone())
            }else {None}
            
        }).collect();
        //we remove all the files that ave been written to and are now considered sent
        for path in toremove{
            files.remove(&path);
        }
        
        //we sleep slightly longer to catch any writes that happen exacty when this runs
        thread::sleep(delay+Duration::from_millis(10))
       }
    
}