use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::Args;

/// Establishes and returns Some serial connection or None.
///
/// # Arguments
///
/// * `args` - The struct Args that contains the CLI options
///
pub fn get_serial_connection(args: &Args) -> Option<SerialStream> {
    let mut serial_connection_res = tokio_serial::new(args.port.clone(), args.baud)
        .data_bits(args.data_bits.value)
        .flow_control(args.flow_control.value)
        .parity(args.parity.value)
        .stop_bits(args.stop_bits.value)
        .open_native_async();

    match serial_connection_res {
        Ok(mut serial_connection) => {
            #[cfg(unix)]
            serial_connection
                .set_exclusive(false)
                .expect("Unable to set serial port exclusive to false");

            Some(serial_connection)
        }
        Err(_) => None,
    }
}
