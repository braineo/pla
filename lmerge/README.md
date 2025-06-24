# Log Merger

## Design

the cli accept a timestamp (optional) and paths to walk through log files


The timestamp is maintained as a cursor state

the cli walk through all the logs in directories like following.


```text
serviceone.log.1
serviceone.log
serviceone.log.2.gz
apiserver_private_access.log.20250624_003014_066242_JST.gz
apiserver_private_access.log.4.gz
apiserver_public_access.log.20250624_034130_049533_JST.gz
apiserver_private_access.log.3.gz
servicetwo.log.20250406_153350_589830_JST
```

It will bucket and index the logs in the following stricture, it should be a very fast processing

``` javascript
[
    {
        name: "serviceone",
        [
            {name: "serviceone.log.1", start: timestamp1, start: timestamp2}
            {name: "serviceone.log.2.gz", start: timestamp1, start: timestamp2}
        ]
    },
    {
        name: "apiserver_private_access",
        [
            {name: "apiserver_private_access.log.20250624_003014_066242_JST.gz", start: timestamp1, start: timestamp2}
            {name: "apiserver_private_access.log.4.gz", start: timestamp1, start: timestamp2}
        ]
    },
    // and more
]
```


Then the cli maintains a sliding windows of a range, like +1/-1 hour.
With the given timestamp, it uses binary search to identify the lines in different logs and combine the lines and sort them into chronicle order, prefix with the bucket name


## Alternatives

Opening logs and get the `head` and `tail` and lines counts are costly. Maybe some of the implementation should be use the cli instead?
