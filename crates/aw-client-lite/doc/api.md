ActivityWatch uses a REST API for all communication between aw-server and clients. Most applications should never use HTTP directly but should instead use the client libraries available. If no such library yet exists for a given language, this document is meant to provide enough specification to create one.

Warning

The API is currently under development, and is subject to change. It will be documented in better detail when first version has been frozen.

Note

Part of the documentation might be outdated, you can get up-to-date API documentation in the API browser available from the web UI of your aw-server instance.

Contents

-   [REST API](https://docs.activitywatch.net/en/latest/api/rest.html#rest-api)
    
    -   [REST Security](https://docs.activitywatch.net/en/latest/api/rest.html#rest-security)
        
    -   [REST Reference](https://docs.activitywatch.net/en/latest/api/rest.html#rest-reference)
        
        -   [Buckets API](https://docs.activitywatch.net/en/latest/api/rest.html#buckets-api)
            
            -   [Get Bucket Metadata](https://docs.activitywatch.net/en/latest/api/rest.html#get-bucket-metadata)
                
            -   [List](https://docs.activitywatch.net/en/latest/api/rest.html#list)
                
            -   [Create](https://docs.activitywatch.net/en/latest/api/rest.html#create)
                
        -   [Events API](https://docs.activitywatch.net/en/latest/api/rest.html#events-api)
            
            -   [Get events](https://docs.activitywatch.net/en/latest/api/rest.html#get-events)
                
            -   [Create event](https://docs.activitywatch.net/en/latest/api/rest.html#create-event)
                
        -   [Heartbeat API](https://docs.activitywatch.net/en/latest/api/rest.html#heartbeat-api)
            
        -   [Query API](https://docs.activitywatch.net/en/latest/api/rest.html#query-api)
            

## [REST Security](https://docs.activitywatch.net/en/latest/api/rest.html#id2)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#rest-security "Permalink to this headline")

Note

Our current security consists only of not allowing non-localhost connections, this is likely to be the case for quite a while.

Clients might in the future be able to have read-only or append-only access to buckets, providing additional security and preventing compromised clients from being able to cause a severe security breach. All clients will probably also encrypt data in transit.

## [REST Reference](https://docs.activitywatch.net/en/latest/api/rest.html#id3)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#rest-reference "Permalink to this headline")

Note

This reference is highly incomplete. For an interactive view of the API, try out the API playground running on your local server at: [http://localhost:5600/api/](http://localhost:5600/api/)

### [Buckets API](https://docs.activitywatch.net/en/latest/api/rest.html#id4)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#buckets-api "Permalink to this headline")

The most common API used by ActivityWatch clients is the API providing read and append access buckets. Buckets are data containers used to group data together which shares some metadata (such as client type, hostname or location).

#### [Get Bucket Metadata](https://docs.activitywatch.net/en/latest/api/rest.html#id5)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#get-bucket-metadata "Permalink to this headline")

Will return 404 if bucket does not exist

```
<span></span>GET<span> </span>/api/0/buckets/&lt;bucket_id&gt;
```

#### [List](https://docs.activitywatch.net/en/latest/api/rest.html#id6)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#list "Permalink to this headline")

#### [Create](https://docs.activitywatch.net/en/latest/api/rest.html#id7)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#create "Permalink to this headline")

Will return 304 if bucket already exists

```
<span></span>POST<span> </span>/api/0/buckets/&lt;bucket_id&gt;
```

### [Events API](https://docs.activitywatch.net/en/latest/api/rest.html#id8)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#events-api "Permalink to this headline")

The most common API used by ActivityWatch clients is the API providing read and append [Events](https://docs.activitywatch.net/en/latest/buckets-and-events.html) to buckets. Buckets are data containers used to group data together which shares some metadata (such as client type, hostname or location).

#### [Get events](https://docs.activitywatch.net/en/latest/api/rest.html#id9)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#get-events "Permalink to this headline")

```
<span></span>GET<span> </span>/api/0/buckets/&lt;bucket_id&gt;/events
```

#### [Create event](https://docs.activitywatch.net/en/latest/api/rest.html#id10)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#create-event "Permalink to this headline")

```
<span></span>POST<span> </span>/api/0/buckets/&lt;bucket_id&gt;/events
```

### [Heartbeat API](https://docs.activitywatch.net/en/latest/api/rest.html#id11)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#heartbeat-api "Permalink to this headline")

The [heartbeat](https://docs.activitywatch.net/en/latest/buckets-and-events.html#heartbeats) API is one of the most useful endpoints for writing watchers.

```
<span></span>POST<span> </span>/api/0/buckets/&lt;bucket_id&gt;/heartbeat
```

### [Query API](https://docs.activitywatch.net/en/latest/api/rest.html#id12)[¶](https://docs.activitywatch.net/en/latest/api/rest.html#query-api "Permalink to this headline")

[Writing Queries](https://docs.activitywatch.net/en/latest/examples/querying-data.html)