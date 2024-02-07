const { readFileSync, writeFileSync } = require('fs')

function fixIndexJs(filename) {
  let content = readFileSync(filename).toString()

  let newContent = `
globalThis.require = require
`
  content += newContent
  content = content.replace("const { join } = require('path')", `const { join } = require('path')
const { inspect } = require('util')
const customInspectSymbol = Symbol.for('nodejs.util.inspect.custom')`)
  writeFileSync(filename, content)
}

function fixIndexDTs(_filename) { }


let filename = process.argv[process.argv.length - 1]
if (filename.endsWith('index.js')) {
  fixIndexJs(filename)
} else if (filename.endsWith('index.d.ts')) {
  fixIndexDTs(filename)
}
