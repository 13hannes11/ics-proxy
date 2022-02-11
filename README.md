# ics-proxy

You can find a running instance of this code on [ics-proxy.de](https://ics-proxy.de). The docker image is published on the [GitHub Container Registry](https://github.com/13hannes11/ics-proxy/pkgs/container/ics-proxy).

## Screenshots

![image](https://user-images.githubusercontent.com/9381167/136559243-2a7c9062-33e3-436e-a781-fef3173e1671.png)
![image](https://user-images.githubusercontent.com/9381167/136559368-3404a94f-35d1-4235-8c98-2f837b75fda0.png)



## Motivation

This application was build because of my frustration with my Universities publishing of schedules. Schedules can be found on [TimeEdit](https://cloud.timeedit.net), 
however, changes to my schedule (for example dropping a course) meant that all ics links on all of my devices needed to be replaced. 
Additionally, each semester this needed to be done as well. I therefore created this proxy to have an easy way of replacing ics links without needing to update the link in my calenders.

## Building

The easiest way to build this repository is to use docker. You can simply run `docker build -t ics-proxy .`

## Deployment

To deploy you can simply use the `docker-compose.yml` file.

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.
