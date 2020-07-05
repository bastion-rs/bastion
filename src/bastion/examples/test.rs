use std::collections::HashMap;
use std::fmt::{self, Debug, Display};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

// ----------------------------------------------------------------

pub trait Request {
    type Response;
}

trait Receiver<M>
where
    M: Debug,
{
    fn receive(&self, message: M);
}

trait RequestHandler<R>
where
    R: Debug + Request,
    R::Response: Debug,
{
    fn handle(&self, request: R) -> Result<R::Response, String>;
}

// ---------------------------------------------------------------

#[derive(Debug)]
struct Room {
    movie: String,
    available_seats: usize,
}

impl Room {
    pub fn new(movie: String, available_seats: usize) -> Self {
        Self {
            movie,
            available_seats,
        }
    }
}

impl Display for Room {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hosting {} with {} available seats",
            self.movie, self.available_seats
        )
    }
}

#[derive(Debug)]
struct Cinema {
    next_ticket_id: AtomicUsize,
    rooms: Mutex<HashMap<String, Room>>,
}

impl Default for Cinema {
    fn default() -> Self {
        Self {
            next_ticket_id: AtomicUsize::new(1),
            rooms: Mutex::new(HashMap::new()),
        }
    }
}

#[derive(Debug)]
struct Reservation {
    name: String,
    movie: String,
}

#[derive(Debug)]
struct BookedTicket {
    name: String,
    seat_number: usize,
    ticket_id: usize,
}

impl Request for Reservation {
    type Response = BookedTicket;
}

impl Display for BookedTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} got seat number {}. Ticket id is {}",
            self.name, self.seat_number, self.ticket_id
        )
    }
}

impl Receiver<Room> for Cinema {
    fn receive(&self, room: Room) {
        println!("Opened new room: {}", room);
        self.rooms.lock().unwrap().insert(room.movie.clone(), room);
    }
}

impl RequestHandler<Reservation> for Cinema {
    fn handle(
        &self,
        reservation: Reservation,
    ) -> Result<<Reservation as Request>::Response, String> {
        let seat_number = self
            .rooms
            .lock()
            .unwrap()
            .get_mut(&reservation.movie)
            .map(|mut room| {
                let seat_number = room.available_seats;

                if seat_number == 0 {
                    Err(format!("no more seats for {} today", reservation.movie))
                } else {
                    room.available_seats -= 1;
                    Ok(seat_number)
                }
            })
            .ok_or_else(|| format!("no room displaying {} today", reservation.movie))??;

        Ok(BookedTicket {
            name: reservation.name,
            seat_number,
            ticket_id: self.next_ticket_id.fetch_add(1, Ordering::SeqCst),
        })
    }
}

// --------------------------------------------------------------

fn main() {
    let cinema = Cinema::default();

    let rooms = vec![
        Room::new("Jurassic Park".to_string(), 10),
        Room::new("Star Wars".to_string(), 50),
        Room::new("Back To The Future".to_string(), 20),
    ];

    for r in rooms {
        cinema.receive(r);
    }

    let tickets = (1..=10usize).map(|i| {
        let r = Reservation {
            name: format!("Jeremy_{}", i),
            movie: "Star Wars".to_string(),
        };

        cinema.handle(r).expect("woopsie")
    });

    for t in tickets {
        println!("{}", t);
    }

    dbg!(&cinema);
}
