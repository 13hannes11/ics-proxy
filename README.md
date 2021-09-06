# ics-proxy

## Motivation

This application was build because of my frustration with my Universities publishing of schedules. Schedules can be found on [TimeEdit](https://cloud.timeedit.net), 
however, changes to my schedule (for example dropping a course) meant that all ics links on all of my devices needed to be replaced. 
Additionally each semester this needed to be done as well. I therefore created this proxy to have an easy way of replaceing ics links without needing to update the link in my callenders.

## Buidling

The easiest way to build this repository is to use docker. You can simply run `docker build -t ics-proxy .`

## Deployment

To deploy you can simply use the `docker-compose.yml` file.