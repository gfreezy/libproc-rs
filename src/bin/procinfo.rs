use std::env;
use std::io::Write;

use libproc;
use libproc::libproc::proc_pid;
use libproc::libproc::proc_pid::{name, pidfdinfo, ProcFDType, SocketFDInfo, SocketInfoKind};
use std::convert::TryInto;

mod c {
    extern crate libc;

    extern "C" {
        pub fn getpid() -> libc::pid_t;
    }
}

fn getpid() -> i32 {
    unsafe { c::getpid() }
}

fn procinfo(pid: i32) {
    match proc_pid::libversion() {
        Ok((major, minor)) => println!("Libversion: {}.{}", major, minor),
        Err(err) => writeln!(&mut std::io::stderr(), "Error: {}", err).unwrap(),
    }

    println!("Pid: {}", pid);

    match proc_pid::pidpath(pid) {
        Ok(path) => println!("Path: {}", path),
        Err(err) => writeln!(&mut std::io::stderr(), "Error: {}", err).unwrap(),
    }

    match proc_pid::name(pid) {
        Ok(name) => println!("Name: {}", name),
        Err(err) => writeln!(&mut std::io::stderr(), "Error: {}", err).unwrap(),
    }

    match proc_pid::regionfilename(pid, 0) {
        Ok(regionfilename) => println!("Region Filename (at address 0): {}", regionfilename),
        Err(err) => writeln!(&mut std::io::stderr(), "Error: {}", err).unwrap(),
    }

    match proc_pid::listpids(proc_pid::ProcType::ProcAllPIDS, 0) {
        Ok(pids) => {
            println!("There are currently {} processes active", pids.len());
            for pid in pids {
                let pid = pid.try_into().unwrap();
                println!("pid: {}, name: {}", pid, name(pid).unwrap());
                for fd in proc_pid::listpidinfo::<proc_pid::ListFDs>(pid, 4000).unwrap() {
                    match fd.proc_fdtype.into() {
                        ProcFDType::Socket => {
                            if let Ok(socket) = pidfdinfo::<SocketFDInfo>(pid, fd.proc_fd) {
                                match socket.psi.soi_kind.into() {
                                    SocketInfoKind::Tcp => {
                                        // access to the member of `soi_proto` is unsafe becasuse of union type.
                                        let info = unsafe { socket.psi.soi_proto.pri_tcp };

                                        // change endian and cut off because insi_lport is network endian and 16bit witdh.
                                        let mut port = 0;
                                        port |= info.tcpsi_ini.insi_lport >> 8 & 0x00ff;
                                        port |= info.tcpsi_ini.insi_lport << 8 & 0xff00;

                                        // access to the member of `insi_laddr` is unsafe becasuse of union type.
                                        let s_addr = unsafe {
                                            info.tcpsi_ini.insi_laddr.ina_46.i46a_addr4.s_addr
                                        };

                                        // change endian because insi_laddr is network endian.
                                        let mut addr = 0;
                                        addr |= s_addr >> 24 & 0x000000ff;
                                        addr |= s_addr >> 8 & 0x0000ff00;
                                        addr |= s_addr << 8 & 0x00ff0000;
                                        addr |= s_addr << 24 & 0xff000000;

                                        println!(
                                            "{}.{}.{}.{}:{}",
                                            addr >> 24 & 0xff,
                                            addr >> 16 & 0xff,
                                            addr >> 8 & 0xff,
                                            addr & 0xff,
                                            port
                                        );
                                    }
                                    _ => (),
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
        Err(err) => writeln!(&mut std::io::stderr(), "Error: {}", err).unwrap(),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let pid;

    if args.len() == 1 {
        pid = getpid();
    } else {
        pid = args[1].clone().parse::<i32>().unwrap();
    }

    procinfo(pid);
}
