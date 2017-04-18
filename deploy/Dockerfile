FROM ubuntu:precise
ENV ROCKET_ENV prod
RUN apt-get update && apt-get install -y libpq-dev
ADD app app
WORKDIR app
ENTRYPOINT ./brdgme-api
EXPOSE 80