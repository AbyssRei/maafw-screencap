// The optional dependency supplies link declarations to the P0 integration
// prototype; it has no capture entry point in this scaffold binary.
#[cfg(all(target_os = "windows", feature = "prototype-windows"))]
use windows_link as _;

fn main() {
    println!("Hello, world!");
}
