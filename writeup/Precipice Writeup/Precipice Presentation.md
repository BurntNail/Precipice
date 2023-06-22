# Precipice

A new solution to allow programmers to quickly profile their programs and gain statistical insights into their execution times.

notes:
In the modern day, many programmers are simply able to settle for the so-called naive solution, thanks to the ever-increasing computing speeds we now live and work with. However, a small sub-set of programmers still do need fast and efficient code, like those working with embedded hardware (which is typically smaller, cheaper, and much less powerful), Operating Systems (which need to take up as few resources as possible) or Games (which need to run faster in real time in conjunction with many other systems ass).  

---
## The plan of attack

 - Architecture
 - Code Samples
 - Regrets/Future Ideas

---

## Architecture

notes:
Now, I could spend some time speaking to you about the underlying architecture, but I'm speaking to a bunch of programmers here and I think we'd all rather dive straight into some code.

---

## Code Samples

![[winky.png|400]]

notes:

Here, I'll focus on a few main samples - for example, I won't be looking at how I get input into the benchmarker unless that code is particularly interesting and none of you want to see my CSV-shuffling code, trust me.

---

### EguiList

![[ijw.gif]]

notes:
I'll now be speaking about *EguiList* which is a tool I use in the GUI parts of my program which allow me to quickly display a list of values without needing to faff around with scrolling or editing code and it all *just works*.

I won't actually be speaking about how it displays elements, just using it as a kinda predecessor and intro to Rust code, of which I will be showing a fair bit during this presentation.

---
#### Enums

```rust
enum Alcohol {
	Vodka,
	Beer,
	Wine
}
```

notes:
Right, so first I'll need to explain enums. Some of you might have encountered them in other languages, and you just think its a list of variants and pretty boring. 

---

#### Unions

```rust
union BitTwiddler {
	bytes: [u8; 16],
	number: u128 //8*16 = 128
}
```

notes:

One thing you most certainly have not encountered are untagged unions (if you have, congrats - you're smart now shut up and let me explain to the heathens). In Rust, a `u` means an positive number, and the number afterwards represents how many bits are used to store it. Here, to store `bytes` we'd need 128 bits, and to store `number`, we'd need 128 bits. The difference between a union and a struct is that in a union, all of the memory is shared so it can only be one kind at a time. This has some incredibly useful applications. This one above is less useful, and just provides a useful way to get 16 bytes as a u128 without a raw memory reinterpretation.

Before you ask, yes this is an incredibly good way to very quickly get memory errors if you forget which variant you're dealing with. However, there exists a way to fix it!

---

#### Rust Enums

```rust
enum ParsedNumber {
	Success(i32), //successful parsed number
	Failure(usize) //index of failure
}

fn parse_int (s: String) -> ParsedNumber;
```

notes:

This is one of the superpowers of rust - we can combine untagged unions and enums to get tagged unions. We take the cost of one extra byte to store which variant we are in, and instantly get to do things like this. This is very useful for errors as it allows the user to quickly grab success values, but forces you to obviously deal with it in a manner very easy to check for in a larger codebase.

---
#### Result
```rust
enum Result<T, E> {
	Ok(T),
	Err(E)
}

fn parse_int (s: String) -> Result<i32, ParseIntError>;

let x = parse_int("abc").unwrap(); //PANICs!!
let y = match parse_int("123") {
	Ok(y) => y,
	Err(e) => {
		eprintln!("Unable to find: {e:?}");
		std::process::exit(1);
	}
}
```

notes:

Here, we've got generics being introduced to the party. Here, `T` and `E` are just types that can be filled later, and we fill them in with the function signature so that T is an i32, and E is a ParseIntError.

Then, below I demonstrate a few ways to get things out of a result. We can unwrap, which just panics (similar to throwing an exception) if it is an `Err`. We can also match on it, which is like a switch statement with superpowers to deal with rust enums.

---

#### Variables
```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChangeType<T> {
	Removed(T),
	Reordered,
}

#[derive(Debug, Clone)]
pub struct EguiList<T> {
	is_scrollable: bool,
	is_editable: bool,
	is_reorderable: bool,
	had_list_update: Option<ChangeType<T>>,
	backing: Vec<T>,
}

```


notes:
Above you can see 1 generic enum which can act as a flag for when items are changed. The generic part means that it can contain any type that has a compile-time available size. I can use an enum here to ensure that invariants are always enforced, like making sure that if it is in the removed state, then there is always an item.

There is also the *EguiList* itself, which contains some variables which act as flags which are determined by the user. The backing list actually contains the items. The list update is then polled by the user to ensure that events are correctly dealt with. This needs to be polled to update the UI as it removes interactivity, whilst there is input that has not been processed by the consumer.

---

#### Traits

```rust
impl<T> Default for EguiList<T> {
	fn default() -> Self {
		Self { is_scrollable: false, is_editable: false,
			is_reorderable: false,
			backing: vec![], had_list_update: None }
	}
}
impl<T> Deref for EguiList<T> {
	type Target = Vec<T>;
	fn deref(&self) -> &Self::Target {
		&self.backing
	}
}
//etc.
```

notes:

Here, I've cut the code down a bit as otherwise it wouldn't fit nicely into the presentation and it is all pretty similar to the above. I have also had to commit formatting crimes to fit the code into the space I've got here.

These are traits - a great feature for Rust, and mirrored in many other languages by *abstract classes* or *interfaces*. (gauge audience, possibly use draw example maybe link back to egui app structure).

Here, I implement a few traits to let any user of the library pretend that they are just dealing with a normal list. Luke - it is specifically *Deref*, *DerefMut*, *AsRef<\[T\]>*, *AsMut<\[T\]>*, *IntoIterator*, and *From<\Vec\>*. 

---

#### From\<Vec\>

```rust
impl<T> From<Vec<T>> for EguiList<T> {
	fn from(value: Vec<T>) -> Self {
		Self {
			backing: value,
			..Default::default()
		}
	}
}
```

notes:
From\<Vec\> is really super cool because it uses the default implementation from earlier where I can tell rust just to use the list passed in as the backing, and then just set everything else to the default that I set - no need to worry about which value is the default like in a langauge like Go - Defaults are opt-in **not** opt-out.

---

### Benchmarker

![[stopwatch-removebg-preview.png]]

notes:

Now, we get to the meat & potatoes of this presentation. I'll be talking about the benchmarker, which is somewhat more sophisticated than the stopwatch pictured above.

---

#### Variables
```rust
enum core::option::Option<T> {
	Some(T),
	None
}

pub struct Runner {
    pub binary: PathBuf,
    pub cli_args: Vec<String>,
    pub runs: usize,
    pub stop_rx: Option<Receiver<()>>,
    pub warmup: u8,
    pub print_initial: bool,
}
```

notes:

Right, now here I've got 2 declarations - an enum and a new struct. The enum is in the standard library and is used for nullable values. The struct (explain values i cba lol)

---

### Benchmark run

![[spaghett-removebg-preview.png]]
notes:

Now, get ready for the real meat & potatoes of this presentation - the actual benchmarker itself. This function gets long and complicated, so I'll be splitting it up into bite-sized pieces.

---

#### Function signature

```rust
pub fn start(self) -> (JoinHandle<io::Result<()>>, Receiver<Duration>) {
    let Self {
        runs,
        binary,
        cli_args,
        stop_rx,
        warmup,
        print_initial,
    } = self;

	//more code ;)
}
```

notes:
Now, I understand that I'm starting simple here, but I've got new syntax and it is a decent place to start.
 - This function takes self, which means that you can accidentally set 2 benchmark threads going unless you mean to.
 - It doesn't return runs, and instead returns a JoinHandle for the thread and a Receiver. That receiver will receive the run times on a separate thread and that gets processed by the UI. Think of a channel as a huge unbounded list that doesn't block so messages can easily be shared between threads - incredibly useful for avoiding obscure memory errors and keeping UI up-to-date. The JoinHandle just lets users join the thread and check when it finishes.

---
#### Threads
```rust
    let (duration_sender, duration_receiver) = channel();
    let handle = Builder::new() //NB: explain
        .name("benchmark_runner".into())
        .spawn(move || {
	        let mut command = Command::new(binary);
			command.args(cli_args);
	        //guess what? more code :D
	        Ok(())
	    }
	    .expect("error creating thread");
    (handle, duration_receiver)
```

notes:

So, here we start the thread up. I give it a name, and move the existing arguments into that thread - this explicit-ness ensures that we avoid data races. I then immediately create the command and set the CLI arguments. I'm not sure if you remember (flick back), but our join handle had a generic type with an IOResult, which means that our handle must return an IOResult. I don't actually need to return anything - I'm just using it for convenient error handling, so I can use the `?` operator. In that case, I can return an empty tuple for ease. Put a pin in that for now. Then, since spawning a thread is a fallible operation, I have to deal with the error. However - since if I can't spawn a thread I'm kinda scuppered, I just panic and exit the process if I can't.

---

#### ?

```rust
fn abc () -> Result<i32, ParseError>;

fn do_stuff_match () -> Result<i32, ParseError> {
	let x = match abc() {
		Ok(worked) => worked,
		Err(e) => return Err(e)
	};

	Ok(x)
}
fn do_stuff_try () -> Result<i32, ParseError> {
	let x = abc()?;
	OK(x)
}
```

notes:

This is another place where rust syntactic sugar comes into play - the ? operator quickly bubbles up errors - these 2 functions are exactly the same.

---

#### Warmup

```rust
if let Ok(cd) = current_dir() {
    command.current_dir(cd);
}
let mut is_first = true;
for _ in 0..warmup {
    let Output {status, stdout, stderr} = command.output()?;
    if !status.success() {
        error!(?status, "Initial Command failed");
        return Ok(());
    }
    //print output
}
command.stdout(Stdio::null()).stderr(Stdio::null());
```

notes:
This is where I handle the warmup. I have also moved the directory faffery here to fit it in the code boxes. The if statement at the top is another rust magic trick - `current_dir` gets the current directory, but could have any number of reasons for failing ranging from non-existent CDs to incorrect permissions so I have to deal with the error. This basically means - if I can get an `Ok` out of the `Result`, then bind the name to `cd` and lemme use it.

I then loop through the warmups, and grab the output. If its the first run and the variable to print the first run was set, then I do that, and then I set `is_first` to false. I always print the errors.

Finally, I redirect the output for the main runs straight into the bin to make sure that printing time doesn't affect the runs - I've had times where my program has been significantly slower because I've been printing often.

---

#### benching code

```rust
for chunk_size in (0..runs).chunks(CHUNK_SIZE)
    .into_iter().map(Iterator::count)
{
    if stop_rx.as_ref()
        .map_or(true, |stop_recv| 
        matches!(stop_recv.try_recv(), Err(TryRecvError::Empty)))
    {
        for _ in 0..chunk_size {
            start = Instant::now();
            let status = command.status()?;
            let elapsed = start.elapsed();
            duration_sender
                .send(elapsed)
                .expect("Error sending result");
  
            if !status.success() {
                warn!(?status, "Command failed");
            }
        }
    } else {
        break;
    }
}
```

notes:

Sorry for the scroll here! So, first I split the runs into chunks, and then for every chunk I grab its length. I then iterate over those lengths.

First thing I do is then to check if we have a stop receiver, and if we do I try to get input from it. If it fails by way of being empty, they haven't sent the stop message yet so we go through to the chunk. If not, then I break the loop, and the thread ends not long after.

Inside the loop, I reset the timer, run the program and get the time it took. I then send it to the sender - the only way it can fail is if the receiver is closed and if it does then I need to get out of the thread so panicking works well. 

If the command fails, I warn in the console, rather than stopping.

---

### Done!

![[all done.jpg]]

notes:

Right, thanks for listening but this presentation is now over. I'll quickly demo the benchmarker and whilst I get it running I'll happily take questions.