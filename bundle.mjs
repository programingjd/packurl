import {build,stop} from 'https://deno.land/x/esbuild/mod.js';

let httpPlugin={
  name: 'http',
  setup(build) {
    build.onResolve({ filter: /^https?:\/\// }, args=>({
      path: args.path,
      namespace: 'http-url',
    }));
    build.onResolve({ filter: /.*/, namespace: 'http-url' }, args=>({
      path: new URL(args.path, args.importer).toString(),
      namespace: 'http-url',
    }))
    build.onLoad({ filter: /.*/, namespace: 'http-url' }, async args=>{
      console.log(args.path);
      const response=await fetch(args.path,{ redirect: 'follow' });
      return { contents: await response.text() };
    })
  },
}

await build({
  entryPoints: ['editor.mjs'],
  outfile: 'editor-bundle.mjs',
  sourcemap: 'linked',
  bundle: true,
  minify: true,
  treeShaking: false,
  format: 'esm',
  plugins: [httpPlugin],
  target: [ 'es2021' ],
});

stop();
