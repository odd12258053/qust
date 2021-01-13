# Qust

Qust is a fast and straightforward in-memory job queue implemented by Rust.

# API

## ADDJOB
Add the job in the given queue.

`ADDJOB <queue name> <retry> <job>`

- queue name: string
    - This param is the name of queue. 
    - If a queue does not exist, Qust creates it.
- retry: integer
    - This param is the time(seconds) until the job is re-queue. 
- job: string
    - This param is job.
    - Job max size is 1MB.

## GETJOB
Get a job with the given queues.
If set multi queues, queues are processed left to right.

`GETJOB <queue name 1> ... <queue name N>`

- queue name: string
    - This param is the name of queue. 

## ACKJOB
Acknowledge the one or more jobs via ID.

`ACKJOB <job id 1> ... <job id N>`

- job id: string
    - This param is the ID of job. This ID has been obtained by the command `GETJOB`.

## STATQUE
TODO

`STATQUE <queue name>`

## DELQUE
TODO

`DELQUE <queue name>`
