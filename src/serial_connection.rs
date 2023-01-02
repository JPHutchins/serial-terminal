use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::Args;

/// Establishes and returns the serial connection.
///
/// # Arguments
///
/// * `args` - The struct Args that contains the CLI options
///
pub fn get_serial_connection(args: Args) -> SerialStream {
    let serial_connection = tokio_serial::new(args.port, args.baud)
        .data_bits(args.data_bits.value)
        .flow_control(args.flow_control.value)
        .parity(args.parity.value)
        .stop_bits(args.stop_bits.value)
        .open_native_async()
        .unwrap();

    #[cfg(unix)]
    serial_connection
        .set_exclusive(false)
        .expect("Unable to set serial port exclusive to false");

    serial_connection
}
