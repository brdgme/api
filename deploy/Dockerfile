FROM ubuntu:precise
RUN apt-get update && apt-get install -y libpq-dev
ADD app app
WORKDIR app
ENTRYPOINT ./brdgme-api
EXPOSE 8000