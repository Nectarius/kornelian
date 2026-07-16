#run application locally
 dx serve --addr 127.0.0.1 --port 8001

 Deployment
To deploy:

Copy .env.example to .env and fill in your actual values
Run docker-compose up --build -d
The application will be accessible at http://localhost on the host machine.


sudo fuser -k 8080/tcp

Execution Commands
To execute, navigate to your project directory /home/nefarius/workspaces_taffeite/taffeite/taffeite and run the following terminal commands:

1. Build the Docker Image
bash
docker build -t kornelian:latest .
2. Option A: Run using docker run
This command automatically loads environment variables from your .env file, overrides the mode to prod, mounts your local TLS certificates, and starts the container in the background:

bash
docker run -d \
  --name kornelian \
  -p 443:443 \
  --env-file .env \
  -e APP_MODE=prod \
  -v "$(pwd)/kornelian.com.pem:/app/kornelian.com.pem" \
  -v "$(pwd)/kornelian.com.key:/app/kornelian.com.key" \
  --restart unless-stopped \
  taffeite:latest
3. Option B: Run using Docker Compose (Recommended)
Docker Compose will automatically pick up and interpolate the environment variables from the .env file located in the same directory:

bash
docker compose up -d

docker build --no-cache -t taffeite:latest .

docker run -p 443:443 taffeite-app

docker-compose up --build -d

docker-compose up --build

docker load -i kornelian.tar


docker save -o kornelian.tar kornelian:latest

docker save -o kornelian.tar taffeite-app:latest



docker run  \
  --name kornelian \
  -p 443:443 \
  --env-file .env \
  -e APP_MODE=prod \
  -v "$(pwd)/kornelian.com.pem:/app/kornelian.com.pem" \
  -v "$(pwd)/kornelian.com.key:/app/kornelian.com.key" \
  --restart unless-stopped \
  kornelian:latest


  cargo build --release --features server

  dx build --release

  assets and index.html
