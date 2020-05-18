use crossbeam_channel::{bounded, Receiver, Sender};
use std::net::{TcpStream, ToSocketAddrs};
use std::{io, io::BufReader, thread};

use crate::stdio::{make_io_threads, IoThreads};
use crate::Message;

pub(crate) fn socket_transport<A: ToSocketAddrs>(
    addr: A,
) -> (Sender<Message>, Receiver<Message>, IoThreads) {
    let stream = TcpStream::connect(addr).expect("Couldn't connect to the server...");
    let (reader_receiver, reader) = make_reader(stream.try_clone().unwrap());
    let (writer_sender, writer) = make_write(stream.try_clone().unwrap());
    let io_threads = make_io_threads(reader, writer);
    (writer_sender, reader_receiver, io_threads)
}

fn make_reader(stream: TcpStream) -> (Receiver<Message>, thread::JoinHandle<io::Result<()>>) {
    let (reader_sender, reader_receiver) = bounded::<Message>(0);
    let reader = thread::spawn(move || {
        let mut buf_read = BufReader::new(stream);
        while let Some(msg) = Message::read(&mut buf_read).unwrap() {
            let is_exit = match &msg {
                Message::Notification(n) => n.is_exit(),
                _ => false,
            };
            reader_sender.send(msg).unwrap();
            if is_exit {
                break;
            }
        }
        Ok(())
    });
    (reader_receiver, reader)
}

fn make_write(mut stream: TcpStream) -> (Sender<Message>, thread::JoinHandle<io::Result<()>>) {
    let (writer_sender, writer_receiver) = bounded::<Message>(0);
    let writer = thread::spawn(move || {
        writer_receiver.into_iter().try_for_each(|it| it.write(&mut stream)).unwrap();
        Ok(())
    });
    (writer_sender, writer)
}
