# packurl

***Use a web url as a self extracting archive.*** 

It's like a zip archive, except it's not a file but a (long) https url, and you don't need any specialized software to extract it, just a web browser.

Even though it's a link to a web page, all the data is contained in the url itself and not hosted on the server.

Also like a zip archive, you have an option to require a password for the decompression.

The unpacking of the content is done by the static site hosted on this repository (work in progress). There are no access logs storing the visited urls.

Urls have a maximum size that is fairly low (2MB for Chrome). To alleviate this constraint, a series of templates are available and can be used to remove all the boilerplate content from the url and replace it with just the template id.

GitHub also limits the url to 8k (the default for nginx) and returns a 414 error when it is larger.
To get around this limitation, a custom 414 page is used. It installs a service worker. Once the service worker is installed, then the subsequent requests go directly through the service worker and are not subject to the limitation. The service worker takes care of updating the decompression page as needed. An added benefit to the use of the service worker is that the url never actually reaches the server and therefore there's no opportunity to log the full url.
