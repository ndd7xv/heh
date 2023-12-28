Thanks for you interest in contributing! Changes of all types are welcome, from feature additions and major code refactoring to the tiniest typo fix. If you want to make change,

1. Fork this repository
2. Make desired changes in (a) descriptive commit(s)
3. Make a PR, linking any issues that may be related

...and follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct) all the way through!

## Goals and Roadmap

This was initially a hobby project, though now I believe it has the potential to be a **fully-fledged terminal hex editor.** There are many improvements that could make using `heh` better. The main problems are:

1. I don't use hex editors often, so I don't know the pain points and wants of a developer who regularly utilizes a hex editor.
1. I have other commitments (and will probably until mid 2025) that keep me busy from contributing as much as I'd like to.

**Because of this, major updates to `heh` will be less frequent.** I will keep versions updated, code review pull requests, and enhance `heh` when I find the time. However, larger quality of life improvements will probably be rare until I'm no longer preoccupied and have the time to carefully research for and develop the terminal hex editor.

I do plan on returning to work on `heh` and open source in general. If you are interested in contributing, I've listed items that I think could use improvement and am willing to elaborate on them if needed (just create a PR or issue). **I'm also open to adding maintainers,** although I'm doubtful finding any for a project I myself cannot actively dedicate time towards.

Any other suggestions and ideas not included are gladly welcome in the form of issues and PRs!

### Ideas

- Crash Handling
- More test coverage
- Tests in a seperate folder
- Better Documentation (what little is written is written by [me](https://github.com/ndd7xv), and there may be some seemingly obvious things I've left out because I can't work on this project with fresh eyes)
- Enforce unsafety documentation through clippy (maybe via [lint configuration in cargo](https://blog.rust-lang.org/2023/11/16/Rust-1.74.0.html#lint-configuration-through-cargo))
- Customizable shortcuts
- More shortcuts (maybe vim like; for example, `gg` or `GG` to go to the top of bottom)
- Explore the use of [ratatui's scrollbar](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Scrollbar.html) (`heh` was made prior to the creation of that widget)
- An investigation into using [Miri](https://github.com/rust-lang/miri) (does running it have any problems)
- Automate releases (currently I just create a release from GitHub in my browser)

## Structure

While not a comprehensive list, some of the pointers may help you get started navigating the code base.

- Each `.rs` file in the `src/windows` represents a different window that can show up -- for example, [`jump_to_byte.rs`](src/windows/jump_to_byte.rs) displays a popup where a user can specify the offset in a file they want to jump to. If you want to add another window:
    - Use the other window.rs files as reference (a relatively simple example is [`unsaved_changes.rs`](src/windows/unsaved_changes.rs))
    - Implement the KeyHandler trait for your window
    - Add your window into the Window enum
    - Modify [`app.rs`](src/app.rs) and [`input.rs`](src/input.rs) to register keys to set the window to the one you created
- At a high level from the [`src/`](src/) directory:
    - [`input.rs`](src/input.rs) is for input handling (i.e. keyboard and mouse)
    - [`screen.rs`](src/screen.rs) renders what is seen in the terminal
    - [`app.rs`](src/app.rs) culminates everything and maintains the state of the application

These 3 files work pretty closely with one another; input from the user modifies the state of the app, and the state of the app is read by the screen to display content to the user.

- Other files include:
    - [`label.rs`](src/label.rs) contain the logic for figuring out the information (Signed 8 bit, Offset, etc) at the cursor
    - [`character.rs`](src/character.rs) converts file bytes into ASCII/Unicode
    - [`buffer.rs`](src/buffer.rs) maintains the relevant part of the file that is being viewed/edited in order to work on large files
    - [`chunk.rs`](src/chunk.rs) and `decoder.rs` are used to read the file and display its contents in hex and ASCII/Unicode

- Run `cargo doc --open` to get a more human readable view of the code descriptions - if you find anything ambiguous, confusing, or obsure, please edit or let me know (doc related or not)!
