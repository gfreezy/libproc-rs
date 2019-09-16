use libproc;
use libproc::libproc::kmesg_buffer;
use std::io::Write;

fn main() {
    if kmesg_buffer::am_root() {
        match kmesg_buffer::kmsgbuf() {
            Ok(message) => println!("{}", message),
            Err(err) => writeln!(&mut std::io::stderr(), "Error: {}", err).unwrap(),
        }
    } else {
        writeln!(&mut std::io::stderr(), "Must be run as root").unwrap()
    }
}
