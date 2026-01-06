Contents

-   [Data model](https://docs.activitywatch.net/en/latest/buckets-and-events.html#data-model)
    
    -   [Buckets](https://docs.activitywatch.net/en/latest/buckets-and-events.html#buckets)
        
    -   [Events](https://docs.activitywatch.net/en/latest/buckets-and-events.html#events)
        
        -   [Heartbeats](https://docs.activitywatch.net/en/latest/buckets-and-events.html#heartbeats)
            
        -   [Event types](https://docs.activitywatch.net/en/latest/buckets-and-events.html#event-types)
            
            -   [web.tab.current](https://docs.activitywatch.net/en/latest/buckets-and-events.html#web-tab-current)
                
            -   [app.editor.activity](https://docs.activitywatch.net/en/latest/buckets-and-events.html#app-editor-activity)
                
            -   [currentwindow](https://docs.activitywatch.net/en/latest/buckets-and-events.html#currentwindow)
                
            -   [afkstatus](https://docs.activitywatch.net/en/latest/buckets-and-events.html#afkstatus)
                

## [Buckets](https://docs.activitywatch.net/en/latest/buckets-and-events.html#id2)[¶](https://docs.activitywatch.net/en/latest/buckets-and-events.html#buckets "Permalink to this headline")

Each bucket contains a series of events and metadata for those events (such as their type and what collected them).

It is recommended to have one bucket per watcher and host. A bucket should always receive data from the same source.

For example, if we want to write a watcher that should track the currently active window we would first have it create a bucket named ‘example-watcher-window\_myhostname’ and then start reporting events to that bucket (using [Heartbeats](https://docs.activitywatch.net/en/latest/buckets-and-events.html#heartbeats)).

```
<span></span><span>bucket</span><span> </span><span>=</span><span> </span><span>{</span>
<span>  </span><span>"id"</span><span>:</span><span> </span><span>"aw-watcher-test_myhostname"</span><span>,</span>
<span>  </span><span>"created"</span><span>:</span><span> </span><span>"2017-05-16T13:37:00.000000"</span><span>,</span>
<span>  </span><span>"name"</span><span>:</span><span> </span><span>"A short but descriptive human readable bucketname"</span><span>,</span>
<span>  </span><span>"type"</span><span>:</span><span> </span><span>"com.example.test"</span><span>,</span><span>       </span><span>// Type of events in bucket</span>
<span>  </span><span>"client"</span><span>:</span><span> </span><span>"example-watcher-test"</span><span>,</span><span> </span><span>// Identifier of client software used to report data</span>
<span>  </span><span>"hostname"</span><span>:</span><span> </span><span>"myhostname"</span><span>,</span><span>         </span><span>// Hostname of device where data was collected</span>
<span>}</span>
```

For information about the “type” field, see examples at [Event types](https://docs.activitywatch.net/en/latest/buckets-and-events.html#event-types).

## [Events](https://docs.activitywatch.net/en/latest/buckets-and-events.html#id3)[¶](https://docs.activitywatch.net/en/latest/buckets-and-events.html#events "Permalink to this headline")

The event model used by ActivityWatch is pretty simple, here is the JSON representation:

```
<span></span><span>event</span><span> </span><span>=</span><span> </span><span>{</span>
<span>  </span><span>"timestamp"</span><span>:</span><span> </span><span>"2016-04-27T15:23:55Z"</span><span>,</span><span>  </span><span>// ISO8601 formatted timestamp</span>
<span>  </span><span>"duration"</span><span>:</span><span> </span><span>3.14</span><span>,</span><span>                     </span><span>// Duration in seconds</span>
<span>  </span><span>"data"</span><span>:</span><span> </span><span>{</span><span>"key"</span><span>:</span><span> </span><span>"value"</span><span>},</span><span>  </span><span>// A JSON object, the schema of this depends on the event type</span>
<span>}</span>
```

It should be noted that all timestamps are stored as UTC. Timezone information (UTC offset) is currently discarded.

The “data” field can be any JSON object, but it is recommended that every event in a bucket should follow some format according to the bucket type, so the data is easy to analyze.

### [Heartbeats](https://docs.activitywatch.net/en/latest/buckets-and-events.html#id4)[¶](https://docs.activitywatch.net/en/latest/buckets-and-events.html#heartbeats "Permalink to this headline")

Heartbeats is a method that merges adjacent events with identical data (within a `pulsetime` window). This is useful to save on storage space and disk IO, and it is recommended that watchers use it when sending events to the server.

A merge of two events A and B is done if their `data` is identical and their timestamps are within the `pulsetime` window. The resulting event will have the earlier timestamp, and a duration to match the difference between the timestamps.

See for example [`aw_transform.heartbeat_merge()`](https://docs.activitywatch.net/en/latest/api/python.html#aw_transform.heartbeat_merge "aw_transform.heartbeat_merge") or the [heartbeat REST API](https://docs.activitywatch.net/en/latest/api/rest.html#heartbeat-api).

### [Event types](https://docs.activitywatch.net/en/latest/buckets-and-events.html#id5)[¶](https://docs.activitywatch.net/en/latest/buckets-and-events.html#event-types "Permalink to this headline")

To separate different types of data in ActivityWatch there is the event type. A buckets event type specified the schema of the events in the bucket.

By creating standards for watchers to use we enable easier transformation and visualization.

#### [web.tab.current](https://docs.activitywatch.net/en/latest/buckets-and-events.html#id6)[¶](https://docs.activitywatch.net/en/latest/buckets-and-events.html#web-tab-current "Permalink to this headline")

An event type for the currently active webbrowser tab.

```
<span></span><span>{</span>
<span>    </span><span>url</span><span>:</span><span> </span><span>string</span><span>,</span>
<span>    </span><span>title</span><span>:</span><span> </span><span>string</span><span>,</span>
<span>    </span><span>audible</span><span>:</span><span> </span><span>bool</span><span>,</span>
<span>    </span><span>incognito</span><span>:</span><span> </span><span>bool</span><span>,</span>
<span>}</span>
```

#### [app.editor.activity](https://docs.activitywatch.net/en/latest/buckets-and-events.html#id7)[¶](https://docs.activitywatch.net/en/latest/buckets-and-events.html#app-editor-activity "Permalink to this headline")

An event type for tracking the currently edited file.

```
<span></span><span>{</span>
<span>    </span><span>file</span><span>:</span><span> </span><span>string</span><span>,</span><span>     </span><span>// full path to file, folders separated by forward slash</span>
<span>    </span><span>project</span><span>:</span><span> </span><span>string</span><span>,</span><span>  </span><span>// full path of cwd, folders separated by forward slash</span>
<span>    </span><span>language</span><span>:</span><span> </span><span>string</span><span>,</span><span> </span><span>// name of language of the file</span>
<span>}</span>
```

#### [currentwindow](https://docs.activitywatch.net/en/latest/buckets-and-events.html#id8)[¶](https://docs.activitywatch.net/en/latest/buckets-and-events.html#currentwindow "Permalink to this headline")

Note

There are suggestions to improve/change this format (see [issue #201](https://github.com/ActivityWatch/activitywatch/issues/201))

```
<span></span><span>{</span>
<span>    </span><span>app</span><span>:</span><span> </span><span>string</span><span>,</span>
<span>    </span><span>title</span><span>:</span><span> </span><span>string</span><span>,</span>
<span>}</span>
```

#### [afkstatus](https://docs.activitywatch.net/en/latest/buckets-and-events.html#id9)[¶](https://docs.activitywatch.net/en/latest/buckets-and-events.html#afkstatus "Permalink to this headline")

Note

There are suggestions to improve/change this format (see [issue #201](https://github.com/ActivityWatch/activitywatch/issues/201))

```
<span></span><span>{</span>
<span>    </span><span>status</span><span>:</span><span> </span><span>string</span><span>   </span><span>// "afk" or "not-afk"</span>
<span>}</span>
```