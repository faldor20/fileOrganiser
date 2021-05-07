use std::{collections::HashMap, convert::TryFrom, env::args, ffi::{OsStr, OsString}, fs::FileType, panic::catch_unwind, path::{self, Path, PathBuf}, str::FromStr, sync::{Arc, mpsc::{Receiver, Sender, channel}}, thread::{self, sleep}, time::{self, Duration, SystemTime}};
use crossbeam::{self, scope};
use notify::*;
mod finished_file_watcher;
use finished_file_watcher::finishedFiles;
use chrono::prelude::*;
use fs_extra::file::{CopyOptions, move_file};
use argh::FromArgs;


//this is a complicated tool that really probably should have just tried to move eveything once every x minutes and not boethered with nay that failed.....
//god damn that would have been so much quicker and easeier

#[derive(FromArgs)]
///Used to take any new files in one directory and move them into another
struct Input{
    ///the source folder to watch for new files
    #[argh(positional)]
    source:PathBuf,

    ///the destination folder to build the folder structire and place new files into
    #[argh(positional)]
    dest:PathBuf,
    ///the amount of inactivity before a fiel is considered done with being written to, should be kept above 10s anything below 5 and things will probably break
    #[argh(positional)]
    delay:u64


}

fn main() {
    let inp: Input = argh::from_env();
    alternative(&inp.source,&inp.dest,Duration::from_secs(inp.delay));
    //defines how long after the last write we wait before the fikle is considered "done being written to"
    println!("Hello, world!");
   
}
fn print_help(){
    println!("
    Sometihg went wrong parsing your input.\n
    Use like: 'dater.exe source dest checkingdelay(s)'\n
    eg: 'dater.exe /my/source/dir /mydest/dir 15
    ")
}
fn run(source:PathBuf,dest:PathBuf,last_write_delay:Duration,){
    let (tx,rx)=channel();
    
    finishedFiles(source.into(), last_write_delay,tx);
    println!("created watching task, now waiting for files");
    rx.iter().for_each(|path|{move_to_dated_folder(path, &dest)})

}
fn make_otuput(source_file:&PathBuf,output_dir:&PathBuf)->PathBuf{
    let local=Local::now();
    let year= local.year();
    let month=local.month();
    let day= local.day();
    let extension=source_file.extension().unwrap_or_default();
    let dest_dir= output_dir.join(year.to_string()).join(month.to_string());
    std::fs::create_dir_all(&dest_dir).expect("couldn't create path. Teason:");
    //we name the file by the day followed by the name
    let filename= format!("{:02}" ,day)+ &*source_file.file_stem().unwrap().to_string_lossy(); //TODO: this is proabbly bad because i am formating it as a debug string
    let dest_path=dest_dir.join(filename).with_extension(extension);
    dest_path
}
fn move_to_dated_folder(source_file:PathBuf,output_dir:&PathBuf){
    
    let dest_path=make_otuput(&source_file, output_dir);
    println!("Moving file from '{:?}' to '{:?}'",source_file,dest_path);
    let mut opts= CopyOptions::new();
    opts.overwrite=true;
    catch_unwind(||{
    move_file(&source_file,dest_path,&opts ).map_err(|er|{println!("couldn't copy file, Reason:{:}",er)});
}).map_err(|err|println!("Failed to move file {:?}, this means it is probabyly in use, it will tried next time. err: {:?}",&source_file,err));
}
fn alternative(source:&PathBuf,dest_dir:&PathBuf,delay:Duration){
    println!("looping infinitley and moving and files from {:?} to a date hierachy at {:?}",source ,dest_dir);
    loop{
        std::fs::read_dir(source).expect(&format!("could not access given directory:{:?}",source))
        .filter_map(|s|s.ok())
        .filter_map(|s|s.file_type().ok().map(|t|(t,s)))
        .for_each(|(f_type,item)|{if f_type.is_file() {
            move_to_dated_folder(item.path(), dest_dir);

        }});
        sleep(delay);
    }
}
