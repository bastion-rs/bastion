Create a conference.

1st Group
Staff (10) - Going to organize the event

2nd Group
Enthusiasts (100) - interested in participating to the conference (haven't registered yet)

3rd Group
Attendees (empty for now) - Participate

Enthusiast -> Ask one of the staff members "when is the conference going to happen ?"
Broadcast / Question => Answer 0 or 1 Staff members are going to reply eventually?

Staff -> Send a Leaflet to all of the enthusiasts, letting them know that they can register.

"hey conference <awesomeconference> is going to happen. will you be there?"
Broadcast / Question -> if people reply with YES => fill the 3rd group
some enthusiasts are now attendees

Staff -> send the actual schedule and misc infos to Attendees
Broadcast / Statement (Attendees)

An attendee sends a thank you note to one staff member (and not bother everyone)
One message / Statement (Staff)

```rust
    let staff = Distributor::named("staff");

    let enthusiasts = Distributor::named("enthusiasts");

    let attendees = Disitributor::named("attendees");

    // Enthusiast -> Ask the whole staff "when is the conference going to happen ?"
    ask_one(Message + Clone) -> Result<impl Future<Output = Reply>, CouldNotSendError>
    // await_one // await_all
    // first ? means "have we been able to send the question?"
    // it s in a month
    let replies = staff.ask_one("when is the conference going to happen ?")?.await?;

    ask_everyone(Message + Clone) -> Result<impl Stream<Item = Reply>, CouldNotSendError>
    let participants = enthusiasts.ask_everyone("here's our super nice conference, it s happening people!").await?;

    for participant in participants {
        // grab the sender and add it to the attendee recipient group
    }

    // send the schedule
    tell_everyone(Message + Clone) -> Result<(), CouldNotSendError>
    attendees.tell_everyone("hey there, conf is in a week, here s where and how it s going to happen")?;

    // send a thank you note
    tell(Message) -> Result<(), CouldNotSendError>
    staff.tell_one("thank's it was amazing")?;

    children
        .with_redundancy(10)
        .with_distributor(Distributor::named("staff"))
        // We create the function to exec when each children is called
        .with_exec(move |ctx: BastionContext| async move { /* ... */ })
    children
        .with_redundancy(100)
        .with_distributor(Distributor::named("enthusiasts"))
        // We create the function to exec when each children is called
        .with_exec(move |ctx: BastionContext| async move { /* ... */ })

    children
        .with_redundancy(0)
        .with_distributor(Distributor::named("attendees"))
        // We create the function to exec when each children is called
        .with_exec(move |ctx: BastionContext| async move { /* ... */ })
```
