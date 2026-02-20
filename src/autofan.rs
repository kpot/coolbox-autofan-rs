use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bus::Bus;
use serialport::{SerialPort, TTYPort};

pub const READ_TIMEOUT_MS: u64 = 500;
pub const POST_CONNECTION_TIMEOUT_MS: u64 = 800;

pub fn open_coolbox_autofan_port(device_path: &str) -> Result<TTYPort, serialport::Error> {
    // The CoolBox board uses 9600 baud, 8N1, no flow control.
    let port = TTYPort::open(
        &serialport::new(device_path, 9600)
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .timeout(Duration::from_millis(READ_TIMEOUT_MS)),
    );
    // Some time is necessary for the device to initialize
    std::thread::sleep(Duration::from_millis(POST_CONNECTION_TIMEOUT_MS));
    port
}

pub struct CoolboxAutofan {
    #[allow(dead_code)]
    tty_port_path: Option<String>,
    tty_port_and_receiver: Mutex<(Box<dyn SerialPort>, std::sync::mpsc::Receiver<String>)>,
    listening_handle: std::thread::JoinHandle<io::Result<()>>,
    listening_exit_flag: Arc<AtomicBool>,
    command_started_flag: Arc<AtomicBool>,
    command_delivered_flag: Arc<AtomicBool>,
    stream_bus: Arc<Mutex<Bus<Vec<u8>>>>,
}

/// Constantly listens for any messages from the Coolbox Autofan Board (CAB).
/// This is one of the most mysterious things in the design of the board: once you disconnect from it,
/// it forgets its last instructions. For instance, if it was ordered to maintain a certain temperature
/// target, once you disconnected it can switch back to manual mode and default speed/temperature settings.
/// So the port has to be always open, ready for reading.
fn listening_thread(
    mut port: Box<dyn SerialPort>,
    exit_flag: Arc<AtomicBool>,
    is_command_started: Arc<AtomicBool>,
    is_command_delivered: Arc<AtomicBool>,
    response_sender: std::sync::mpsc::SyncSender<String>,
    stream_bus: Arc<Mutex<Bus<Vec<u8>>>>,
) -> io::Result<()> {
    fn dump_broadcast_buffer(
        stream_bus: &Arc<Mutex<Bus<Vec<u8>>>>,
        broadcast_buffer: &mut Vec<u8>,
    ) {
        if broadcast_buffer.len() > 0 {
            if let Ok(mut bus) = stream_bus.lock() {
                let to_broadcast = std::mem::take(broadcast_buffer);
                bus.broadcast(to_broadcast);
            }
        }
    }

    fn dump_command_buffer(
        command_started_flag: &Arc<AtomicBool>,
        is_command_delivered: &Arc<AtomicBool>,
        response_sender: &std::sync::mpsc::SyncSender<String>,
        command_buffer: &mut Vec<u8>,
    ) {
        let response = String::from_utf8_lossy(&*command_buffer).to_string();
        response_sender.send(response.clone()).ok();
        command_buffer.clear();
        command_started_flag.store(false, Ordering::Relaxed);
        is_command_delivered.store(false, Ordering::Relaxed);
    }

    let mut device_buffer: [u8; 1] = [0; 1];
    let mut broadcast_buffer = Vec::<u8>::new();
    let mut command_buffer = Vec::<u8>::new();
    loop {
        if exit_flag.load(Ordering::Relaxed) {
            log::info!("Stopping listener as requested");
            return Ok(());
        }
        match port.read(&mut device_buffer) {
            Ok(bytes) => {
                broadcast_buffer.extend_from_slice(&device_buffer);
                if bytes == 0 {
                    // EOF. Time to dump whatever we've read so far
                    dump_broadcast_buffer(&stream_bus, &mut broadcast_buffer);
                }
                if is_command_started.load(Ordering::Relaxed) {
                    if bytes == 1 {
                        command_buffer.extend_from_slice(&device_buffer);
                    } else {
                        if is_command_delivered.load(Ordering::Relaxed) {
                            // EOF. Time to dump whatever we've read so far
                            dump_command_buffer(
                                &is_command_started,
                                &is_command_delivered,
                                &response_sender,
                                &mut command_buffer,
                            );
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                // Timeout is also a sign we can dump whatever we've read so far if we have some
                if is_command_started.load(Ordering::Relaxed)
                    && is_command_delivered.load(Ordering::Relaxed)
                {
                    dump_command_buffer(
                        &is_command_started,
                        &is_command_delivered,
                        &response_sender,
                        &mut command_buffer,
                    );
                }
                dump_broadcast_buffer(&stream_bus, &mut broadcast_buffer);
            }
            Err(e) => {
                log::error!("Unable to read from the device: {:?}", e);
                return Err(e);
            }
        }
    }
}

impl TryFrom<String> for CoolboxAutofan {
    type Error = serialport::Error;

    fn try_from(tty_port_path: String) -> Result<Self, Self::Error> {
        let tty_port = open_coolbox_autofan_port(&tty_port_path)?;
        let listening_port_clone = tty_port.try_clone()?;
        Ok(Self::from_ports(
            Box::new(tty_port),
            listening_port_clone,
            Some(tty_port_path),
        ))
    }
}

impl CoolboxAutofan {
    #[allow(dead_code)]
    pub fn join(self) -> io::Result<()> {
        self.listening_exit_flag.store(true, Ordering::Relaxed);
        match self.listening_handle.join() {
            Ok(result) => result,
            Err(..) => Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Listening thread has panicked",
            )),
        }
    }

    pub fn dummy() -> Result<Self, serialport::Error> {
        let (tty_port, listening_port_clone) = TTYPort::pair()?;
        let listening_port_clone = Box::new(listening_port_clone);
        Ok(Self::from_ports(
            Box::new(tty_port),
            listening_port_clone,
            None,
        ))
    }

    pub fn from_ports(
        writing_port: Box<dyn serialport::SerialPort>,
        reading_port: Box<dyn serialport::SerialPort>,
        tty_port_path: Option<String>,
    ) -> Self {
        let listening_exit_flag = Arc::new(AtomicBool::new(false));
        let listening_exit_flag_clone = Arc::clone(&listening_exit_flag);

        let command_started_flag = Arc::new(AtomicBool::new(false));
        let command_started_clone = Arc::clone(&command_started_flag);

        let command_delivered_flag = Arc::new(AtomicBool::new(false));
        let command_delivered_clone = Arc::clone(&command_delivered_flag);

        let (response_sender, response_receiver) = std::sync::mpsc::sync_channel::<String>(1);

        let stream_bus = Arc::new(Mutex::new(Bus::new(100)));
        let stream_bus_clone = Arc::clone(&stream_bus);

        let listening_handle = std::thread::spawn(move || -> Result<(), io::Error> {
            listening_thread(
                reading_port,
                listening_exit_flag_clone,
                command_started_clone,
                command_delivered_clone,
                response_sender,
                stream_bus_clone,
            )
        });

        Self {
            listening_handle,
            listening_exit_flag,
            command_started_flag,
            command_delivered_flag,
            tty_port_path,
            tty_port_and_receiver: Mutex::new((writing_port, response_receiver)),
            stream_bus,
        }
    }

    pub fn send_command(&self, cmd: &[u8]) -> io::Result<String> {
        // Just before writing anything, we set a flag "command is executing".
        // This flag means that all the following bytes coming from the device must be
        // interpreted as a command's reply, and accumulated.
        // Once the accumulation is over (after `read` returns a timeout of meets an EOF),
        // the flag goes down.
        let mut port_and_receiver = self
            .tty_port_and_receiver
            .lock()
            .expect("The lock must be accessible");
        self.command_started_flag.store(true, Ordering::Relaxed);
        port_and_receiver.0.write_all(cmd)?;
        std::thread::sleep(Duration::from_millis(200));
        self.command_delivered_flag.store(true, Ordering::Relaxed);
        log::debug!("Delivered command: {:?}", String::from_utf8_lossy(cmd));
        match port_and_receiver
            .1
            .recv_timeout(Duration::from_millis(1000))
        {
            Ok(response) => Ok(response),
            Err(..) => {
                log::error!("Unable to receive a command's response from the listener");
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Unable to receive command's response from the listener",
                ))
            }
        }
    }

    pub fn subscribe(&self) -> bus::BusReader<Vec<u8>> {
        self.stream_bus.lock().unwrap().add_rx()
    }

    pub fn is_listener_alive(&self) -> bool {
        !self.listening_handle.is_finished()
    }

    pub fn device_path(&self) -> Option<&str> {
        self.tty_port_path.as_deref()
    }
}
