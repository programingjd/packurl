const prefix='packurl-v';
const version=1;
self.addEventListener('install',e=>{
  self.skipWaiting();
  e.waitUntil(install());
});
self.addEventListener('activate',e=>{
  clients.claim();
  e.waitUntil(clearOldCaches());
});
self.addEventListener('fetch',e=>{

});
async function clearOldCaches(){
  const keys=await caches.keys();
  await Promise.all(
    keys.filter(it=>it.startsWith(prefix)).map(async it=>await caches.delete(it))
  );
}
async function install(){
  const cache=await caches.open(`${prefix}${version}`);
}
