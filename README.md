# 📧 Tempinbox.es

Full-stack Rust web application providing instant, free Outlook & Hotmail accounts for trusted, privacy-focused burner emails.

## If you like this project, please drop a star! ⭐️

## Features

- Generate temporary Outlook and Hotmail email addresses
- Generate single or bulk (10) email accounts
- Toggle between username:password and refreshToken:clientID

## How It Works

Tempinbox.es uses advanced techniques to generate temporary email addresses, including:

- Custom FunCaptcha bypass, with 2000 raw recognitions/day provided by [NopeCHA](https://nopecha.com/pricing)
- Rotating residential proxies
- Extremely efficient, low-level API interactions for Microsoft Auth/Funcaptcha!

## Tech Stack

- Frontend: Rust with Yew framework
- Backend: Rust
- Deployment: Vercel

## Getting Started

### Prerequisites

- Rust
- Cargo
- Trunk (for building the Yew application)

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/AmirAgassi/tempinbox.es.git
   cd tempinbox.es
   ```

2. Install dependencies:
   ```
   cargo build
   ```

3. Run the development server:
   ```
   trunk serve
   ```

4. Open your browser and navigate to `http://localhost:8080`

## Deployment

This project is configured for deployment on Vercel. The `vercel.json` file and `build.sh` script handle the build and deployment process.