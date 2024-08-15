# pcf-wasm

serve: `yarn serve`
build: 
```
wasm-pack build --scope=mgtd --target=web
cd pkg
npm publish --access=public
```
