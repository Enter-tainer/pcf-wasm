// Note that a dynamic `import` statement here is required due to
// webpack/webpack#6615, but in theory `import { greet } from './pkg';`
// will work here one day as well!
const rust = import('./pkg');
rust
  .then(m => {
    let res = m.get_pc(BigInt("0b1100001111111000111111110011111110001111"), "JISZ")
    console.log(res)
  })
  .catch(console.error);
