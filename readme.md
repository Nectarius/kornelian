#run application locally
 dx serve --addr 127.0.0.1 --port 8001

 Deployment
To deploy:

Copy .env.example to .env and fill in your actual values
Run docker-compose up --build -d
The application will be accessible at http://localhost on the host machine.