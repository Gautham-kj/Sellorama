# Sellorama

Welcome to Sellorama, an innovative e-commerce platform designed to streamline the process of buying and selling merchandise. Our backend infrastructure is expertly developed in Rust, utilizing PostgreSQL for advanced database management. The Axum Web Framework enhances our backend API, while SQLx ensures seamless database connectivity. For object storage, AWS S3 SDK is employed, with CloudFlare R2 providing object storage solutions in production environments. Sellorama is dedicated to offering a smooth and efficient user experience. Discover our API and access detailed documentation at https://sr-api.gauthamk.xyz/docs .

## Environment Setup

To prepare your development environment:

1. Obtain AWS SDK Credentials, following the guidance in the .env.sample file.
2. Configure your database with your credentials.
3. Set the port and local address for hosting the API on your system.
   Detailed configuration instructions can be found in the [sample format](/.env.sample).
4. Execute `docker-compose up` to launch the service.

## Database Overview

Explore the database architecture by viewing the ER Diagram available at https://dbdiagram.io/d/66707081a179551be6158614.
