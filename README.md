<div align="center">
  <h1>Ruddit</h1>
</div>

**Ruddit** is a command-line (CLI) application for interacting with Reddit, built with Rust.

## ✨ Features

- **Reddit API Interaction**: Connects to the Reddit API to fetch and interact with data.
- **Command-Line Interface**: All operations are performed through a comprehensive set of commands.
- **Database Storage**: Uses a local SQLite database to store data.
- **Data Export**: Export data to CSV and Excel formats.
- **Secure API Key Management**: Securely stores and manages your Reddit API keys.

## 🚀 Installation

To install Ruddit, you need to have Rust and Cargo installed. If you don't, follow the instructions on the [official Rust website](https://www.rust-lang.org/tools/install).

Once Rust is set up, clone the repository and install the application:

```bash
git clone <repository_url>
cd ruddit
cargo install --path .
```

This will install the `ruddit` executable in your Cargo bin directory (usually `~/.cargo/bin`), making it available from anywhere in your terminal.

## ⚙️ Configuration

Before using Ruddit, you need to configure your Reddit API keys.

1. **Create a Reddit App**: Go to your [Reddit apps](https://www.reddit.com/prefs/apps) page and create a new "script" app.
2. **Set API Keys**: Use the following command to set your API keys:

   ```bash
   ruddit --api-key <your_client_id> --api-secret <your_client_secret>
   ```

## 💻 Usage

Detailed usage instructions will be added here as the application develops.

## 🛠️ Technologies Used

- [Rust](https://www.rust-lang.org/)
- [Reqwest](https://docs.rs/reqwest/latest/reqwest/) (for HTTP requests to the Reddit API)
- [Serde](https://serde.rs/) (for serialization/deserialization)
- [Tokio](https://tokio.rs/) (for asynchronous operations)
- [Clap](https://docs.rs/clap/latest/clap/) (for argument parsing)
- [Rusqlite](https://docs.rs/rusqlite/latest/rusqlite/) (for SQLite database)
- [Dotenv](https://crates.io/crates/dotenv) (for environment variables)
- [Chrono](https://docs.rs/chrono/latest/chrono/) (for date and time)
- [TOML](https://docs.rs/toml/latest/toml/) (for configuration file parsing)
- [Rust XlsxWriter](https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/) (for writing Excel files)

## 🙌 Contributing

Contributions are welcome! If you have ideas for new features or find a bug, please open an issue or submit a pull request.

## 📄 License

This project is licensed under the MIT License.

