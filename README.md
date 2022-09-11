# packurl

***Use a web url as a self extracting archive.*** 

It's like a zip archive, except it's not a file but a (long) https url, and you don't need any specialized software to extract it, just a web browser.

Even though it's a link to a web page, all the data is contained in the url itself and not hosted on the server.

Also like a zip archive, you have an option to require a password for the decompression.

It is recommended to visit the root of the domain ([packurl.net](https://packurl.net)) once before visiting an archive url. This installs the service worker that is responsible for unpacking the archives. After that, all archive urls will be handled completely by the service worker locally on the client and the archive url will not even be sent to the server.

If you visit an archive url first without the service worker installed, then the server will stop reading the request as soon as it can determine that this is an archive url, and it will return an error page and stop the connection. This way the rest of the url will not be sent. The error page will install the service worker, and after a refresh, the url will be handled locally.

Urls have a maximum size that is fairly low (2MB for Chrome). To alleviate this constraint, a series of templates are available and can be used to remove all the boilerplate content from the url and replace it with just the template id.
