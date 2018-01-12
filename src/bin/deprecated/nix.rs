#![allow(dead_code, unused_variables, unused_imports)]
extern crate libc;
extern crate nix;
extern crate tempdir;
extern crate tempfile;

use nix::fcntl::{open, openat, readlink, readlinkat, OFlag};
use nix::sys::stat::Mode;
use nix::unistd::{close, read};
use nix::errno::errno;
use tempdir::TempDir;
use tempfile::NamedTempFile;
use std::io::prelude::*;
use std::os::unix::fs;
use std::os::unix::process::CommandExt;

// 1. wait for a special time
// 2. execvp

use std::ffi::CString;
use libc::*;

fn run_cmd() {
    // FIXME error, don't know why
    use std::ptr;
    // let argvs = vec![CString::new("/usr/bin/printenv").unwrap(), CString::new("MY_VAR").unwrap()];
    // let argvs = vec![CString::new("MY_VAR").unwrap().as_ptr(), ptr::null()];
    let argvs = vec![
        CString::new(" hello").unwrap().as_ptr(),
        CString::new(" world").unwrap().as_ptr(),
        ptr::null(),
    ];
    // let argv_ptr: *const *const c_char = {
    //     println!("argv={:?}", argvs);
    //     let mut p_argv: Vec<_> = argvs.iter().map(|a| a.as_ptr()).collect();
    //     p_argv.push(ptr::null());
    //     let p: *const *const c_char = p_argv.as_ptr();
    //     p
    // };
    let argv_ptr = argvs.as_ptr();
    let envps = vec![CString::new("MY_VAR=lol").unwrap().as_ptr(), ptr::null()];
    // let envps = vec![CString::new("MY_VAR=lol").unwrap().as_ptr(), ptr::null()];
    // let envps = vec![];
    // let env_ptr: *const *const c_char = {
    //     println!("envp={:?}", envps);
    //     let mut p_argv: Vec<_> = envps.iter().map(|a| a.as_ptr()).collect();
    //     p_argv.push(ptr::null());
    //     let p: *const *const c_char = p_argv.as_ptr();
    //     p
    // };
    let env_ptr = envps.as_ptr();
    let bin = CString::new("./a.out").unwrap();
    let bin_ptr = bin.as_ptr();
    unsafe {
        execve(bin_ptr, argv_ptr, env_ptr);
    }
    rperror("execve");
}

fn run_it() {
    use std::process::{Command, Stdio};
    let r = Command::new("./a.out")
        .args(&["MY_VAR"])
        .env("MY_VAR", "lol")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .exec();
    println!("==>{:?}", r);
}

fn rperror(s: &str) {
    let cstr = CString::new(s).unwrap();
    let pc = cstr.into_raw();
    unsafe {
        perror(pc);
    }
}

fn fork_child() -> pid_t {
    unsafe {
        let p = fork();
        if p == -1 {
            rperror("fork");
            exit(1);
        }
        if p == 0 {
            println!("child working...");
            run_it();
            // run_cmd();
            sleep(2);
            println!("child exit");
            _exit(0);
        }
        return p;
    }
}

fn main() {
    run();
}

fn run() {
    unsafe {
        let mut mask: sigset_t = std::mem::uninitialized();
        let mut orig_mask: sigset_t = std::mem::uninitialized();
        let timeout: timespec = timespec {
            tv_sec: 1,
            tv_nsec: 0,
        };

        let mask_ptr: *mut sigset_t = &mut mask;
        let orig_mask_ptr: *mut sigset_t = &mut orig_mask;

        sigemptyset(mask_ptr);
        sigaddset(mask_ptr, SIGCHLD);

        if sigprocmask(SIG_BLOCK, mask_ptr, orig_mask_ptr) < 0 {
            rperror("sigprocmask");
            exit(1);
        }

        let pid = fork_child();

        let timeout_ptr: *const timespec = &timeout;

        use std::ptr;

        loop {
            if sigtimedwait(mask_ptr, ptr::null_mut(), timeout_ptr) < 0 {
                if Errno::from_i32(errno()) == Errno::EINTR {
                    continue;
                } else if Errno::from_i32(errno()) == Errno::EAGAIN {
                    println!("timeout, killing child");
                    kill(pid, SIGUSR1);
                } else {
                    rperror("sigtimedwait");
                    exit(1);
                }
            }
            break;
        }

        let mut status: c_int = ::std::mem::uninitialized();
        let status_ptr: *mut c_int = &mut status;

        if waitpid(pid, status_ptr, 0) < 0 {
            rperror("waitpid");
            exit(1);
        }

        println!("status: {}", status);
    }
}