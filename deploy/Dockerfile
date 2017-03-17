FROM amazonlinux
ENV ROCKET_ENV=prod
ADD app app
WORKDIR app
ENTRYPOINT ./brdgme-web
EXPOSE 80