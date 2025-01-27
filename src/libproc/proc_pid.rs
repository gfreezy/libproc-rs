use libc::{
    c_char, c_int, c_short, c_uchar, c_ushort, c_void, gid_t, in6_addr, in_addr, off_t,
    sockaddr_un, uid_t, IF_NAMESIZE, SOCK_MAXADDRLEN,
};
use std::io::{Error, ErrorKind, Result};
use std::mem;
use std::ptr;

// Since we cannot access C macros for constants from Rust - I have had to redefine this, based on Apple's source code
// See http://opensource.apple.com/source/Libc/Libc-594.9.4/darwin/libproc.c
// buffersize must be more than PROC_PIDPATHINFO_SIZE
// buffersize must be less than PROC_PIDPATHINFO_MAXSIZE
//
// See http://opensource.apple.com//source/xnu/xnu-1456.1.26/bsd/sys/proc_info.h
// #define PROC_PIDPATHINFO_SIZE		(MAXPATHLEN)
// #define PROC_PIDPATHINFO_MAXSIZE	(4*MAXPATHLEN)
// in http://opensource.apple.com//source/xnu/xnu-1504.7.4/bsd/sys/param.h
// #define	MAXPATHLEN	PATH_MAX
// in https://opensource.apple.com/source/xnu/xnu-792.25.20/bsd/sys/syslimits.h
// #define	PATH_MAX		 1024
pub const MAXPATHLEN: usize = 1024;
pub const PROC_PIDPATHINFO_MAXSIZE: usize = 4 * MAXPATHLEN;

// from http://opensource.apple.com//source/xnu/xnu-1456.1.26/bsd/sys/proc_info.h
const MAXTHREADNAMESIZE: usize = 64;

// From http://opensource.apple.com//source/xnu/xnu-1456.1.26/bsd/sys/proc_info.h and
// http://fxr.watson.org/fxr/source/bsd/sys/proc_info.h?v=xnu-2050.18.24
#[derive(Copy, Clone)]
pub enum ProcType {
    ProcAllPIDS = 1,
    ProcPGRPOnly = 2,
    ProcTTYOnly = 3,
    ProcUIDOnly = 4,
    ProcRUIDOnly = 5,
    ProcPPIDOnly = 6,
}

// from http://opensource.apple.com//source/xnu/xnu-1504.7.4/bsd/sys/param.h
const MAXCOMLEN: usize = 16;

// This trait is needed for polymorphism on pidinfo types, also abstracting flavor in order to provide
// type-guaranteed flavor correctness
pub trait PIDInfo: Default {
    fn flavor() -> PidInfoFlavor;
}

// structures from http://opensource.apple.com//source/xnu/xnu-1456.1.26/bsd/sys/proc_info.h
#[repr(C)]
#[derive(Default)]
pub struct TaskInfo {
    pub pti_virtual_size: u64,
    // virtual memory size (bytes)
    pub pti_resident_size: u64,
    // resident memory size (bytes)
    pub pti_total_user: u64,
    // total time
    pub pti_total_system: u64,
    pub pti_threads_user: u64,
    // existing threads only
    pub pti_threads_system: u64,
    pub pti_policy: i32,
    // default policy for new threads
    pub pti_faults: i32,
    // number of page faults
    pub pti_pageins: i32,
    // number of actual pageins
    pub pti_cow_faults: i32,
    // number of copy-on-write faults
    pub pti_messages_sent: i32,
    // number of messages sent
    pub pti_messages_received: i32,
    // number of messages received
    pub pti_syscalls_mach: i32,
    // number of mach system calls
    pub pti_syscalls_unix: i32,
    // number of unix system calls
    pub pti_csw: i32,
    // number of context switches
    pub pti_threadnum: i32,
    // number of threads in the task
    pub pti_numrunning: i32,
    // number of running threads
    pub pti_priority: i32, // task priority
}

impl PIDInfo for TaskInfo {
    fn flavor() -> PidInfoFlavor {
        PidInfoFlavor::TaskInfo
    }
}

#[repr(C)]
#[derive(Default)]
pub struct BSDInfo {
    pub pbi_flags: u32,
    // 64bit; emulated etc
    pub pbi_status: u32,
    pub pbi_xstatus: u32,
    pub pbi_pid: u32,
    pub pbi_ppid: u32,
    pub pbi_uid: uid_t,
    pub pbi_gid: gid_t,
    pub pbi_ruid: uid_t,
    pub pbi_rgid: gid_t,
    pub pbi_svuid: uid_t,
    pub pbi_svgid: gid_t,
    pub rfu_1: u32,
    // reserved
    pub pbi_comm: [c_char; MAXCOMLEN],
    pub pbi_name: [c_char; 2 * MAXCOMLEN],
    // empty if no name is registered
    pub pbi_nfiles: u32,
    pub pbi_pgid: u32,
    pub pbi_pjobc: u32,
    pub e_tdev: u32,
    // controlling tty dev
    pub e_tpgid: u32,
    // tty process group id
    pub pbi_nice: i32,
    pub pbi_start_tvsec: u64,
    pub pbi_start_tvusec: u64,
}

impl PIDInfo for BSDInfo {
    fn flavor() -> PidInfoFlavor {
        PidInfoFlavor::TBSDInfo
    }
}

#[repr(C)]
#[derive(Default)]
pub struct TaskAllInfo {
    pub pbsd: BSDInfo,
    pub ptinfo: TaskInfo,
}

impl PIDInfo for TaskAllInfo {
    fn flavor() -> PidInfoFlavor {
        PidInfoFlavor::TaskAllInfo
    }
}

#[repr(C)]
pub struct ThreadInfo {
    pub pth_user_time: u64,
    // user run time
    pub pth_system_time: u64,
    // system run time
    pub pth_cpu_usage: i32,
    // scaled cpu usage percentage
    pub pth_policy: i32,
    // scheduling policy in effect
    pub pth_run_state: i32,
    // run state (see below)
    pub pth_flags: i32,
    // various flags (see below)
    pub pth_sleep_time: i32,
    // number of seconds that thread
    pub pth_curpri: i32,
    // cur priority
    pub pth_priority: i32,
    // priority
    pub pth_maxpriority: i32,
    // max priority
    pub pth_name: [c_char; MAXTHREADNAMESIZE], // thread name, if any
}

impl PIDInfo for ThreadInfo {
    fn flavor() -> PidInfoFlavor {
        PidInfoFlavor::ThreadInfo
    }
}

impl Default for ThreadInfo {
    fn default() -> ThreadInfo {
        ThreadInfo {
            pth_user_time: 0,
            pth_system_time: 0,
            pth_cpu_usage: 0,
            pth_policy: 0,
            pth_run_state: 0,
            pth_flags: 0,
            pth_sleep_time: 0,
            pth_curpri: 0,
            pth_priority: 0,
            pth_maxpriority: 0,
            pth_name: [0; MAXTHREADNAMESIZE],
        }
    }
}

#[derive(Default)]
pub struct WorkQueueInfo {
    pub pwq_nthreads: u32,
    // total number of workqueue threads
    pub pwq_runthreads: u32,
    // total number of running workqueue threads
    pub pwq_blockedthreads: u32,
    // total number of blocked workqueue threads
    pub reserved: [u32; 1], // reserved for future use
}

impl PIDInfo for WorkQueueInfo {
    fn flavor() -> PidInfoFlavor {
        PidInfoFlavor::WorkQueueInfo
    }
}

// From http://opensource.apple.com/source/xnu/xnu-1504.7.4/bsd/kern/proc_info.c
pub enum PidInfoFlavor {
    ListFDs = 1,
    // list of ints?
    TaskAllInfo = 2,
    // struct proc_taskallinfo
    TBSDInfo = 3,
    // struct proc_bsdinfo
    TaskInfo = 4,
    // struct proc_taskinfo
    ThreadInfo = 5,
    // struct proc_threadinfo
    ListThreads = 6,
    // list if int thread ids
    RegionInfo = 7,
    RegionPathInfo = 8,
    // string?
    VNodePathInfo = 9,
    // string?
    ThreadPathInfo = 10,
    // String?
    PathInfo = 11,
    // String
    WorkQueueInfo = 12, // struct proc_workqueueinfo
}

pub enum PidInfo {
    ListFDs(Vec<i32>),
    // File Descriptors used by Process
    TaskAllInfo(TaskAllInfo),
    TBSDInfo(BSDInfo),
    TaskInfo(TaskInfo),
    ThreadInfo(ThreadInfo),
    ListThreads(Vec<i32>),
    // thread ids
    RegionInfo(String),
    // String??
    RegionPathInfo(String),
    VNodePathInfo(String),
    ThreadPathInfo(String),
    PathInfo(String),
    WorkQueueInfo(WorkQueueInfo),
}

pub enum PidFDInfoFlavor {
    VNodeInfo = 1,
    VNodePathInfo = 2,
    SocketInfo = 3,
    PSEMInfo = 4,
    PSHMInfo = 5,
    PipeInfo = 6,
    KQueueInfo = 7,
    ATalkInfo = 8,
}

// this extern block links to the libproc library
// Original signatures of functions can be found at http://opensource.apple.com/source/Libc/Libc-594.9.4/darwin/libproc.c
#[link(name = "proc", kind = "dylib")]
extern "C" {
    fn proc_listpids(proc_type: u32, typeinfo: u32, buffer: *mut c_void, buffersize: u32) -> c_int;

    fn proc_pidinfo(
        pid: c_int,
        flavor: c_int,
        arg: u64,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;

    fn proc_pidfdinfo(
        pid: c_int,
        fd: c_int,
        flavor: c_int,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;

    fn proc_name(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;

    fn proc_regionfilename(pid: c_int, address: u64, buffer: *mut c_void, buffersize: u32)
        -> c_int;

    fn proc_pidpath(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;

    fn proc_libversion(major: *mut c_int, minor: *mut c_int) -> c_int;
}

/// Returns the PIDs of the processes active that match the ProcType passed in
///
/// # Examples
///
/// ```
/// use std::io::Write;
/// use libproc::libproc::proc_pid;
///
/// match proc_pid::listpids(proc_pid::ProcType::ProcAllPIDS, 0) {
///     Ok(pids) => {
///         assert!(pids.len() > 1);
///         println!("Found {} processes using listpids()", pids.len());
///     }
///     Err(err) => assert!(false, "Error listing pids")
/// }
/// ```
pub fn listpids(proc_types: ProcType, info: u32) -> Result<Vec<u32>> {
    let buffer_size = unsafe { proc_listpids(proc_types as u32, info, ptr::null_mut(), 0) };
    if buffer_size <= 0 {
        return Err(Error::last_os_error());
    }

    let capacity = buffer_size as usize / mem::size_of::<u32>();
    let mut pids: Vec<u32> = Vec::with_capacity(capacity);
    let buffer_ptr = pids.as_mut_ptr() as *mut c_void;

    let ret = unsafe { proc_listpids(proc_types as u32, info, buffer_ptr, buffer_size as u32) };

    if ret <= 0 {
        Err(Error::last_os_error())
    } else {
        let items_count = (ret as usize / mem::size_of::<u32>())
            .checked_sub(1)
            .unwrap_or(0);
        unsafe {
            pids.set_len(items_count);
        }

        Ok(pids)
    }
}

/// Returns the PIDs of the process that match pid passed in.
///
/// arg - is "geavily not documented" and need to look at code for each flavour here
/// http://opensource.apple.com/source/xnu/xnu-1504.7.4/bsd/kern/proc_info.c
/// to figure out what it's doing.... Pull-Requests welcome!
///
/// # Examples
///
/// ```
/// use std::io::Write;
/// use libproc::libproc::proc_pid::{pidinfo, BSDInfo};
///
/// fn pidinfo_test() {
///     use std::process;
///     let pid = process::id() as i32;
///
///     match pidinfo::<BSDInfo>(pid, 0) {
///         Ok(info) => assert_eq!(info.pbi_pid as i32, pid),
///         Err(err) => assert!(false, "Error retrieving process info: {}", err)
///     };
/// }
/// ```
///
pub fn pidinfo<T: PIDInfo>(pid: i32, arg: u64) -> Result<T> {
    let flavor = T::flavor() as i32;
    let buffer_size = mem::size_of::<T>() as i32;
    let mut pidinfo = T::default();
    let buffer_ptr = &mut pidinfo as *mut _ as *mut c_void;
    let ret: i32;

    unsafe {
        ret = proc_pidinfo(pid, flavor, arg, buffer_ptr, buffer_size);
    };

    if ret <= 0 {
        Err(Error::last_os_error())
    } else {
        Ok(pidinfo)
    }
}

pub fn regionfilename(pid: i32, address: u64) -> Result<String> {
    let mut regionfilenamebuf: Vec<u8> = Vec::with_capacity(PROC_PIDPATHINFO_MAXSIZE - 1);
    let buffer_ptr = regionfilenamebuf.as_mut_ptr() as *mut c_void;
    let buffer_size = regionfilenamebuf.capacity() as u32;
    let ret: i32;

    unsafe {
        ret = proc_regionfilename(pid, address, buffer_ptr, buffer_size);
    };

    if ret <= 0 {
        Err(Error::last_os_error())
    } else {
        unsafe {
            regionfilenamebuf.set_len(ret as usize);
        }

        match String::from_utf8(regionfilenamebuf) {
            Ok(regionfilename) => Ok(regionfilename),
            Err(e) => Err(Error::new(
                ErrorKind::Other,
                format!("Invalid UTF-8 sequence: {}", e),
            )),
        }
    }
}

pub fn pidpath(pid: i32) -> Result<String> {
    let mut pathbuf: Vec<u8> = Vec::with_capacity(PROC_PIDPATHINFO_MAXSIZE - 1);
    let buffer_ptr = pathbuf.as_mut_ptr() as *mut c_void;
    let buffer_size = pathbuf.capacity() as u32;
    let ret: i32;

    unsafe {
        ret = proc_pidpath(pid, buffer_ptr, buffer_size);
    };

    if ret <= 0 {
        Err(Error::last_os_error())
    } else {
        unsafe {
            pathbuf.set_len(ret as usize);
        }

        match String::from_utf8(pathbuf) {
            Ok(path) => Ok(path),
            Err(e) => Err(Error::new(
                ErrorKind::Other,
                format!("Invalid UTF-8 sequence: {}", e),
            )),
        }
    }
}

/// Returns the major and minor version numbers of the native librproc library being used
///
/// # Examples
///
/// ```
/// use std::io::Write;
/// use libproc::libproc::proc_pid;
///
/// match proc_pid::libversion() {
///     Ok((major, minor)) => println!("Libversion: {}.{}", major, minor),
///     Err(err) => writeln!(&mut std::io::stderr(), "Error: {}", err).unwrap()
/// }
/// ```
pub fn libversion() -> Result<(i32, i32)> {
    let mut major = 0;
    let mut minor = 0;
    let ret: i32;

    unsafe {
        ret = proc_libversion(&mut major, &mut minor);
    };

    // return value of 0 indicates success (inconsistent with other functions... :-( )
    if ret == 0 {
        Ok((major, minor))
    } else {
        Err(Error::last_os_error())
    }
}

/// Returns the name of the process with the specified pid
///
/// # Examples
///
/// ```
/// use std::io::Write;
/// use libproc::libproc::proc_pid;
///
/// match proc_pid::name(1) {
///     Ok(name) => println!("Name: {}", name),
///     Err(err) => writeln!(&mut std::io::stderr(), "Error: {}", err).unwrap()
/// }
/// ```
pub fn name(pid: i32) -> Result<String> {
    let mut namebuf: Vec<u8> = Vec::with_capacity(PROC_PIDPATHINFO_MAXSIZE - 1);
    let buffer_ptr = namebuf.as_ptr() as *mut c_void;
    let buffer_size = namebuf.capacity() as u32;
    let ret: i32;

    unsafe {
        ret = proc_name(pid, buffer_ptr, buffer_size);
    };

    if ret <= 0 {
        Err(Error::last_os_error())
    } else {
        unsafe {
            namebuf.set_len(ret as usize);
        }

        match String::from_utf8(namebuf) {
            Ok(name) => Ok(name),
            Err(e) => Err(Error::new(
                ErrorKind::Other,
                format!("Invalid UTF-8 sequence: {}", e),
            )),
        }
    }
}

// This trait is needed for polymorphism on listpidinfo types, also abstracting flavor in order to provide
// type-guaranteed flavor correctness
pub trait ListPIDInfo {
    type Item;
    fn flavor() -> PidInfoFlavor;
}

/// Returns the information of the process that match pid passed in.
/// `max_len` is the maximum number of array to return.
/// The length of return value: `Vec<T::Item>` may be less than `max_len`.
///
/// # Examples
///
/// ```
/// use std::io::Write;
/// use libproc::libproc::proc_pid::{listpidinfo, pidinfo, ListFDs, TaskAllInfo, ProcFDType};
///
/// fn listpidinfo_test() {
///     use std::process;
///     let pid = process::id() as i32;
///
///     if let Ok(info) = pidinfo::<TaskAllInfo>(pid, 0) {
///         if let Ok(fds) = listpidinfo::<ListFDs>(pid, info.pbsd.pbi_nfiles as usize) {
///             for fd in &fds {
///                 let fd_type = ProcFDType::from(fd.proc_fdtype);
///                 println!("File Descriptor: {}, Type: {:?}", fd.proc_fd, fd_type);
///             }
///         }
///     }
/// }
/// ```
pub fn listpidinfo<T: ListPIDInfo>(pid: i32, max_len: usize) -> Result<Vec<T::Item>> {
    assert!(max_len <= PROC_PIDPATHINFO_MAXSIZE);
    let flavor = T::flavor() as i32;
    let buffer_size = mem::size_of::<T::Item>() as i32 * max_len as i32;
    let mut buffer = Vec::<T::Item>::with_capacity(max_len);
    let buffer_ptr = unsafe {
        buffer.set_len(max_len);
        buffer.as_mut_ptr() as *mut c_void
    };

    let ret: i32;

    unsafe {
        ret = proc_pidinfo(pid, flavor, 0, buffer_ptr, buffer_size);
    };

    if ret < 0 {
        Err(Error::last_os_error())
    } else if ret == 0 {
        Ok(vec![])
    } else {
        let actual_len = ret as usize / mem::size_of::<T::Item>();
        buffer.truncate(actual_len);
        Ok(buffer)
    }
}

pub struct ListThreads;

impl ListPIDInfo for ListThreads {
    type Item = u64;
    fn flavor() -> PidInfoFlavor {
        PidInfoFlavor::ListThreads
    }
}

pub struct ListFDs;

impl ListPIDInfo for ListFDs {
    type Item = ProcFDInfo;
    fn flavor() -> PidInfoFlavor {
        PidInfoFlavor::ListFDs
    }
}

#[repr(C)]
pub struct ProcFDInfo {
    pub proc_fd: i32,
    pub proc_fdtype: u32,
}

#[derive(Copy, Clone, Debug)]
pub enum ProcFDType {
    /// AppleTalk
    ATalk = 0,
    /// Vnode
    VNode = 1,
    /// Socket
    Socket = 2,
    /// POSIX shared memory
    PSHM = 3,
    /// POSIX semaphore
    PSEM = 4,
    /// Kqueue
    KQueue = 5,
    /// Pipe
    Pipe = 6,
    /// FSEvents
    FSEvents = 7,
    /// Unknown
    Unknown,
}

impl From<u32> for ProcFDType {
    fn from(value: u32) -> ProcFDType {
        match value {
            0 => ProcFDType::ATalk,
            1 => ProcFDType::VNode,
            2 => ProcFDType::Socket,
            3 => ProcFDType::PSHM,
            4 => ProcFDType::PSEM,
            5 => ProcFDType::KQueue,
            6 => ProcFDType::Pipe,
            7 => ProcFDType::FSEvents,
            _ => ProcFDType::Unknown,
        }
    }
}

// This trait is needed for polymorphism on pidfdinfo types, also abstracting flavor in order to provide
// type-guaranteed flavor correctness
pub trait PIDFDInfo: Default {
    fn flavor() -> PidFDInfoFlavor;
}

/// Returns the information about file descriptors of the process that match pid passed in.
///
/// # Examples
///
/// ```
/// use std::io::Write;
/// use std::net::TcpListener;
/// use libproc::libproc::proc_pid::{listpidinfo, pidinfo, pidfdinfo, ListFDs, ListThreads, BSDInfo, ProcFDType, SocketFDInfo, SocketInfoKind};
///
/// fn pidfdinfo_test() {
///     use std::process;
///     let pid = process::id() as i32;
///
///     // Open TCP port:8000 to test.
///     let _listener = TcpListener::bind("127.0.0.1:8000");
///
///     if let Ok(info) = pidinfo::<BSDInfo>(pid, 0) {
///         if let Ok(fds) = listpidinfo::<ListFDs>(pid, info.pbi_nfiles as usize) {
///             for fd in &fds {
///                 match fd.proc_fdtype.into() {
///                     ProcFDType::Socket => {
///                         if let Ok(socket) = pidfdinfo::<SocketFDInfo>(pid, fd.proc_fd) {
///                             match socket.psi.soi_kind.into() {
///                                 SocketInfoKind::Tcp => {
///                                     // access to the member of `soi_proto` is unsafe becasuse of union type.
///                                     let info = unsafe { socket.psi.soi_proto.pri_tcp };
///
///                                     // change endian and cut off because insi_lport is network endian and 16bit witdh.
///                                     let mut port = 0;
///                                     port |= info.tcpsi_ini.insi_lport >> 8 & 0x00ff;
///                                     port |= info.tcpsi_ini.insi_lport << 8 & 0xff00;
///
///                                     // access to the member of `insi_laddr` is unsafe becasuse of union type.
///                                     let s_addr = unsafe { info.tcpsi_ini.insi_laddr.ina_46.i46a_addr4.s_addr };
///
///                                     // change endian because insi_laddr is network endian.
///                                     let mut addr = 0;
///                                     addr |= s_addr >> 24 & 0x000000ff;
///                                     addr |= s_addr >> 8  & 0x0000ff00;
///                                     addr |= s_addr << 8  & 0x00ff0000;
///                                     addr |= s_addr << 24 & 0xff000000;
///
///                                     println!("{}.{}.{}.{}:{}", addr >> 24 & 0xff, addr >> 16 & 0xff, addr >> 8 & 0xff, addr & 0xff, port);
///                                 }
///                                 _ => (),
///                             }
///                         }
///                     }
///                     _ => (),
///                 }
///             }
///         }
///     }
/// }
/// ```
///
pub fn pidfdinfo<T: PIDFDInfo>(pid: i32, fd: i32) -> Result<T> {
    let flavor = T::flavor() as i32;
    let buffer_size = mem::size_of::<T>() as i32;
    let mut pidinfo = T::default();
    let buffer_ptr = &mut pidinfo as *mut _ as *mut c_void;
    let ret: i32;

    unsafe {
        ret = proc_pidfdinfo(pid, fd, flavor, buffer_ptr, buffer_size);
    };

    if ret <= 0 {
        Err(Error::last_os_error())
    } else {
        Ok(pidinfo)
    }
}

#[repr(C)]
#[derive(Default)]
pub struct SocketFDInfo {
    pub pfi: ProcFileInfo,
    pub psi: SocketInfo,
}

impl PIDFDInfo for SocketFDInfo {
    fn flavor() -> PidFDInfoFlavor {
        PidFDInfoFlavor::SocketInfo
    }
}

#[repr(C)]
#[derive(Default)]
pub struct ProcFileInfo {
    pub fi_openflags: u32,
    pub fi_status: u32,
    pub fi_offset: off_t,
    pub fi_type: i32,
    pub rfu_1: i32,
}

#[derive(Copy, Clone, Debug)]
pub enum SocketInfoKind {
    Generic = 0,
    /// IPv4 and IPv6 Sockets
    In = 1,
    /// TCP Sockets
    Tcp = 2,
    /// Unix Domain Sockets
    Un = 3,
    /// PF_NDRV Sockets
    Ndrv = 4,
    /// Kernel Event Sockets
    KernEvent = 5,
    /// Kernel Control Sockets
    KernCtl = 6,
    /// Unknown
    Unknown,
}

impl From<c_int> for SocketInfoKind {
    fn from(value: c_int) -> SocketInfoKind {
        match value {
            0 => SocketInfoKind::Generic,
            1 => SocketInfoKind::In,
            2 => SocketInfoKind::Tcp,
            3 => SocketInfoKind::Un,
            4 => SocketInfoKind::Ndrv,
            5 => SocketInfoKind::KernEvent,
            6 => SocketInfoKind::KernCtl,
            _ => SocketInfoKind::Unknown,
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct SocketInfo {
    pub soi_stat: VInfoStat,
    pub soi_so: u64,
    pub soi_pcb: u64,
    pub soi_type: c_int,
    pub soi_protocol: c_int,
    pub soi_family: c_int,
    pub soi_options: c_short,
    pub soi_linger: c_short,
    pub soi_state: c_short,
    pub soi_qlen: c_short,
    pub soi_incqlen: c_short,
    pub soi_qlimit: c_short,
    pub soi_timeo: c_short,
    pub soi_error: c_ushort,
    pub soi_oobmark: u32,
    pub soi_rcv: SockBufInfo,
    pub soi_snd: SockBufInfo,
    pub soi_kind: c_int,
    pub rfu_1: u32,
    pub soi_proto: SocketInfoProto,
}

#[repr(C)]
#[derive(Default)]
pub struct VInfoStat {
    pub vst_dev: u32,
    pub vst_mode: u16,
    pub vst_nlink: u16,
    pub vst_ino: u64,
    pub vst_uid: uid_t,
    pub vst_gid: gid_t,
    pub vst_atime: i64,
    pub vst_atimensec: i64,
    pub vst_mtime: i64,
    pub vst_mtimensec: i64,
    pub vst_ctime: i64,
    pub vst_ctimensec: i64,
    pub vst_birthtime: i64,
    pub vst_birthtimensec: i64,
    pub vst_size: off_t,
    pub vst_blocks: i64,
    pub vst_blksize: i32,
    pub vst_flags: u32,
    pub vst_gen: u32,
    pub vst_rdev: u32,
    pub vst_qspare: [i64; 2],
}

#[repr(C)]
#[derive(Default)]
pub struct SockBufInfo {
    pub sbi_cc: u32,
    pub sbi_hiwat: u32,
    pub sbi_mbcnt: u32,
    pub sbi_mbmax: u32,
    pub sbi_lowat: u32,
    pub sbi_flags: c_short,
    pub sbi_timeo: c_short,
}

#[repr(C)]
pub union SocketInfoProto {
    pub pri_in: InSockInfo,
    pub pri_tcp: TcpSockInfo,
    pub pri_un: UnSockInfo,
    pub pri_ndrv: NdrvInfo,
    pub pri_kern_event: KernEventInfo,
    pub pri_kern_ctl: KernCtlInfo,
}

impl Default for SocketInfoProto {
    fn default() -> SocketInfoProto {
        SocketInfoProto {
            pri_in: Default::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct In4In6Addr {
    pub i46a_pad32: [u32; 3],
    pub i46a_addr4: in_addr,
}

impl Default for In4In6Addr {
    fn default() -> In4In6Addr {
        In4In6Addr {
            i46a_pad32: [0; 3],
            i46a_addr4: in_addr { s_addr: 0 },
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct InSockInfo {
    pub insi_fport: c_int,
    pub insi_lport: c_int,
    pub insi_gencnt: u64,
    pub insi_flags: u32,
    pub insi_flow: u32,
    pub insi_vflag: u8,
    pub insi_ip_ttl: u8,
    pub rfu_1: u32,
    pub insi_faddr: InSIAddr,
    pub insi_laddr: InSIAddr,
    pub insi_v4: InSIV4,
    pub insi_v6: InSIV6,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct InSIV4 {
    pub in4_top: c_uchar,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct InSIV6 {
    pub in6_hlim: u8,
    pub in6_cksum: c_int,
    pub in6_ifindex: c_ushort,
    pub in6_hops: c_short,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union InSIAddr {
    pub ina_46: In4In6Addr,
    pub ina_6: in6_addr,
}

impl Default for InSIAddr {
    fn default() -> InSIAddr {
        InSIAddr {
            ina_46: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum TcpSIState {
    /// Closed
    Closed = 0,
    /// Listening for connection
    Listen = 1,
    /// Active, have sent syn
    SynSent = 2,
    /// Have send and received syn
    SynReceived = 3,
    /// Established
    Established = 4,
    /// Rcvd fin, waiting for close
    CloseWait = 5,
    /// Have closed, sent fin
    FinWait1 = 6,
    /// Closed xchd FIN; await FIN ACK
    Closing = 7,
    /// Had fin and close; await FIN ACK
    LastAck = 8,
    /// Have closed, fin is acked
    FinWait2 = 9,
    /// In 2*msl quiet wait after close
    TimeWait = 10,
    /// Pseudo state: reserved
    Reserved = 11,
    /// Unknown
    Unknown,
}

impl From<c_int> for TcpSIState {
    fn from(value: c_int) -> TcpSIState {
        match value {
            0 => TcpSIState::Closed,
            1 => TcpSIState::Listen,
            2 => TcpSIState::SynSent,
            3 => TcpSIState::SynReceived,
            4 => TcpSIState::Established,
            5 => TcpSIState::CloseWait,
            6 => TcpSIState::FinWait1,
            7 => TcpSIState::Closing,
            8 => TcpSIState::LastAck,
            9 => TcpSIState::FinWait2,
            10 => TcpSIState::TimeWait,
            11 => TcpSIState::Reserved,
            _ => TcpSIState::Unknown,
        }
    }
}

const TSI_T_NTIMERS: usize = 4;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct TcpSockInfo {
    pub tcpsi_ini: InSockInfo,
    pub tcpsi_state: c_int,
    pub tcpsi_timer: [c_int; TSI_T_NTIMERS],
    pub tcpsi_mss: c_int,
    pub tcpsi_flags: u32,
    pub rfu_1: u32,
    pub tcpsi_tp: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct UnSockInfo {
    pub unsi_conn_so: u64,
    pub unsi_conn_pcb: u64,
    pub unsi_addr: UnSIAddr,
    pub unsi_caddr: UnSIAddr,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union UnSIAddr {
    pub ua_sun: sockaddr_un,
    pub ua_dummy: [c_char; SOCK_MAXADDRLEN as usize],
}

impl Default for UnSIAddr {
    fn default() -> UnSIAddr {
        UnSIAddr {
            ua_dummy: [0; SOCK_MAXADDRLEN as usize],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct NdrvInfo {
    pub ndrvsi_if_family: u32,
    pub ndrvsi_if_unit: u32,
    pub ndrvsi_if_name: [c_char; IF_NAMESIZE],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct KernEventInfo {
    pub kesi_vendor_code_filter: u32,
    pub kesi_class_filter: u32,
    pub kesi_subclass_filter: u32,
}

const MAX_KCTL_NAME: usize = 96;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct KernCtlInfo {
    pub kcsi_id: u32,
    pub kcsi_reg_unit: u32,
    pub kcsi_flags: u32,
    pub kcsi_recvbufsize: u32,
    pub kcsi_sendbufsize: u32,
    pub kcsi_unit: u32,
    pub kcsi_name: [c_char; MAX_KCTL_NAME],
}

impl Default for KernCtlInfo {
    fn default() -> KernCtlInfo {
        KernCtlInfo {
            kcsi_id: 0,
            kcsi_reg_unit: 0,
            kcsi_flags: 0,
            kcsi_recvbufsize: 0,
            kcsi_sendbufsize: 0,
            kcsi_unit: 0,
            kcsi_name: [0; MAX_KCTL_NAME],
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn listpids_test() {
        match listpids(ProcType::ProcAllPIDS, 0) {
            Ok(pids) => {
                assert!(pids.len() > 1);
                println!("Found {} processes using listpids()", pids.len());
            }
            Err(err) => assert!(false, "Error listing pids: {}", err),
        }
    }

    #[test]
    fn listpids_uid_test() {
        let uid = unsafe { libc::getuid() };
        match listpids(ProcType::ProcUIDOnly, uid) {
            Ok(pids) => {
                assert!(pids.len() > 2);
                println!("Found {} processes using listpids(uid)", pids.len());
            }
            Err(err) => assert!(false, "Error listing pids: {}", err),
        }
    }

    #[test]
    fn pidinfo_test() {
        use std::process;
        let pid = process::id() as i32;

        match pidinfo::<BSDInfo>(pid, 0) {
            Ok(info) => assert_eq!(info.pbi_pid as i32, pid),
            Err(err) => assert!(false, "Error retrieving process info: {}", err),
        };
    }

    #[test]
    // This checks that it can find the regionfilename of the region at address 0, of the init process with PID 1
    fn regionfilename_test() {
        match regionfilename(1, 0) {
            // run tests with 'cargo test -- --nocapture' to see the test output
            Ok(regionfilename) => println!(
                "Region Filename (at address = 0) of init process PID = 1 is '{}'",
                regionfilename
            ),
            Err(message) => assert!(true, message),
        }
    }

    #[test]
    // This checks that it can find the path of the init process with PID 1
    fn pidpath_test_init_pid() {
        match pidpath(1) {
            // run tests with 'cargo test -- --nocapture' to see the test output
            Ok(path) => println!("Path of init process with PID = 1 is '{}'", path),
            Err(message) => assert!(false, message),
        }
    }

    #[test]
    #[should_panic]
    // This checks that it cannot find the path of the process with pid -1
    fn pidpath_test_unknown_pid() {
        match pidpath(-1) {
            // run tests with 'cargo test -- --nocapture' to see the test output
            Ok(path) => assert!(
                false,
                "It found the path of process Pwith ID = -1 (path = {}), that's not possible\n",
                path
            ),
            Err(message) => assert!(false, message),
        }
    }

    #[test]
    fn libversion_test() {
        match libversion() {
            Ok((major, minor)) => {
                // run tests with 'cargo test -- --nocapture' to see the test output
                println!("Major = {}, Minor = {}", major, minor);
            }
            Err(message) => assert!(false, message),
        }
    }

    #[test]
    // error: Process didn't exit successfully: `/Users/andrew/workspace/libproc-rs/target/debug/libproc-503ad0ba07eb6318` (signal: 11, SIGSEGV: invalid memory reference)
    // This checks that it can find the name of the init process with PID 1
    fn name_test_init_pid() {
        match pidpath(1) {
            // run tests with 'cargo test -- --nocapture' to see the test output
            Ok(path) => println!("Name of init process PID = 1 is '{}'", path),
            Err(message) => assert!(true, message),
        }
    }

    #[test]
    fn listpidinfo_test() {
        use std::process;
        let pid = process::id() as i32;

        match pidinfo::<TaskAllInfo>(pid, 0) {
            Ok(info) => {
                match listpidinfo::<ListThreads>(pid, info.ptinfo.pti_threadnum as usize) {
                    Ok(threads) => assert!(threads.len() > 0),
                    Err(err) => assert!(false, "Error retrieving process info: {}", err),
                }
                match listpidinfo::<ListFDs>(pid, info.pbsd.pbi_nfiles as usize) {
                    Ok(fds) => assert!(fds.len() > 0),
                    Err(err) => assert!(false, "Error retrieving process info: {}", err),
                }
            }
            Err(err) => assert!(false, "Error retrieving process info: {}", err),
        };
    }

    #[test]
    fn pidfdinfo_test() {
        use std::net::TcpListener;
        use std::process;
        let pid = process::id() as i32;

        let _listener = TcpListener::bind("127.0.0.1:65535");

        let info = pidinfo::<BSDInfo>(pid, 0).unwrap();
        let fds = listpidinfo::<ListFDs>(pid, info.pbi_nfiles as usize).unwrap();
        for fd in fds {
            match fd.proc_fdtype.into() {
                ProcFDType::Socket => {
                    let socket = pidfdinfo::<SocketFDInfo>(pid, fd.proc_fd).unwrap();
                    match socket.psi.soi_kind.into() {
                        SocketInfoKind::Tcp => unsafe {
                            let info = socket.psi.soi_proto.pri_tcp;
                            assert_eq!(socket.psi.soi_protocol, libc::IPPROTO_TCP);
                            assert_eq!(info.tcpsi_ini.insi_lport as u32, 65535);
                        },
                        _ => (),
                    }
                }
                _ => (),
            }
        }
    }
}
