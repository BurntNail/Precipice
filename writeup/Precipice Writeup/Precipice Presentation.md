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

