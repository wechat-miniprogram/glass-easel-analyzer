/* eslint-disable no-console */
import * as fs from 'node:fs'
import path from 'node:path'

const mdnDir = path.join(__dirname, 'mdn')

// utils
const extractContentBetween = (
  fullContent: string,
  startSign: string,
  endSign: string,
): string | null => {
  const startPre = fullContent.indexOf(startSign)
  if (startPre < 0) return null
  const start = startPre + startSign.length
  const end = fullContent.indexOf(endSign, start)
  if (end < 0) return null
  return fullContent.slice(start, end)
}
type CascadeListItem = {
  content: string
  children: CascadeListItem[]
}
const extractCascadeLists = (fullContent: string): CascadeListItem[] => {
  const ret: CascadeListItem[] = []
  const stack: { indent: number; list: CascadeListItem[] }[] = [{ indent: -1, list: ret }]
  fullContent.split('\n').forEach((line) => {
    let indent = 0
    while (indent < line.length) {
      if (line[indent] === ' ') {
        indent += 1
        continue
      }
      if (line[indent] === '-' && line[indent + 1] === ' ') break
      return
    }
    if (indent === line.length) return
    const newItem = {
      content: line.slice(indent + 2),
      children: [],
    }
    let depth = stack.length - 1
    while (stack[depth]!.indent >= indent) {
      depth -= 1
    }
    stack[depth]!.list.push(newItem)
    stack.length = depth + 1
    stack.push({ indent, list: newItem.children })
  })
  return ret
}

// write header
const outFile = fs.openSync(path.join(__dirname, 'web.toml'), 'w', 0o666)
fs.writeSync(
  outFile,
  '# This file is auto generated from [MDN](https://github.com/mdn/content/blob/main).\n',
)
fs.writeSync(
  outFile,
  '# The content is published under [CC-BY-SA 2.5](https://creativecommons.org/licenses/by-sa/2.5/).\n',
)
fs.writeSync(
  outFile,
  '# See the license in [LICENSE](https://github.com/mdn/content/blob/main/LICENSE.md).\n',
)
fs.writeSync(outFile, '\n')

// enumerate global attributes
fs.writeSync(outFile, `\n`)
const globalAttrDir = path.join(mdnDir, 'files/en-us/web/html/global_attributes')
const extractGlobalAttrFromContent = (attrName: string, content: string) => {
  const description =
    extractContentBetween(content, '{{HTMLSidebar("Global_attributes")}}\n\n', '\n\n') ??
    extractContentBetween(
      content,
      '{{HTMLSidebar("Global_attributes")}}{{SeeCompatTable}}\n\n',
      '\n\n',
    ) ??
    extractContentBetween(
      content,
      '{{HTMLSidebar("Global_attributes")}}{{Non-standard_Header}}{{SeeCompatTable}}\n\n',
      '\n\n',
    )
  if (description === null) {
    console.error(`Cannot find description for global attribute "${attrName}". Skipped this tag.`)
    return
  }
  const reference = `https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/${attrName}`
  console.info(`Global Attribute: ${attrName}`)
  fs.writeSync(outFile, '[[global-attribute]]\n')
  fs.writeSync(outFile, `name = "${attrName}"\n`)
  fs.writeSync(outFile, `description = '''${description}'''\n`)
  fs.writeSync(outFile, `reference = "${reference}"\n`)
  fs.writeSync(outFile, `\n`)
}
fs.readdirSync(globalAttrDir).forEach((attrName) => {
  if (attrName.indexOf('.') >= 0) return
  if (attrName.startsWith('data-')) return
  if (attrName === 'is') return
  if (attrName === 'id') return
  if (attrName === 'slot') return
  if (attrName === 'class') return
  if (attrName === 'style') return
  const elementPath = path.join(globalAttrDir, attrName, 'index.md')
  const content = fs.readFileSync(elementPath, { encoding: 'utf8' })
  extractGlobalAttrFromContent(attrName, content)
})

// enumerate events
fs.writeSync(outFile, `\n`)
const apiDir = path.join(mdnDir, 'files/en-us/web/api')
type EventDesc = {
  name: string
  description: string
  reference: string
}
const extractEventList = (content: string): EventDesc[] => {
  const ret: EventDesc[] = []
  const section = extractContentBetween(content, '## Events\n\n', '\n\n## ')
  if (section !== null) {
    const attrSegs = extractCascadeLists(section)
    attrSegs.forEach((seg) => {
      let relPath = ''
      let evName = ''
      const args =
        extractContentBetween(seg.content, '{{DOMxRef(', ')}}') ??
        extractContentBetween(seg.content, '{{domxref(', ')}}')
      if (args !== null) {
        const argList = args.split(',')
        if (argList.length < 2) {
          console.error(`Cannot find a proper event name in line "- ${seg.content}".`)
          return
        }
        relPath =
          extractContentBetween(argList[0]!.trim(), '"', '"') ??
          extractContentBetween(argList[0]!.trim(), "'", "'") ??
          ''
        evName =
          extractContentBetween(argList[1]!.trim(), '"', '"') ??
          extractContentBetween(argList[1]!.trim(), "'", "'") ??
          ''
      } else {
        evName = extractContentBetween(content, '`', '`') ?? ''
        relPath = extractContentBetween(content, '](', ')') ?? ''
      }
      if (!relPath || !evName) {
        if (!seg.content.includes('{{Deprecated_Inline}}')) {
          console.error(`Cannot find a proper event in line "- ${seg.content}".`)
        }
        return
      }
      const attrDescLine = seg.children[0]
      if (!attrDescLine || !attrDescLine.content.startsWith(': ')) {
        console.error(
          `Cannot find a proper event content for event "${evName}". Skipped this event.`,
        )
        return
      }
      const description = attrDescLine.content.slice(2)
      ret.push({
        name: evName,
        description,
        reference: `https://developer.mozilla.org/en-US/docs/Web/API/${relPath}`,
      })
    })
  }
  return ret
}
const elementEventMap = Object.create(null) as { [tag: string]: EventDesc[] }
fs.readdirSync(apiDir).forEach((fileName) => {
  if (fileName === 'htmlelement' || fileName === 'element') {
    const elementPath = path.join(apiDir, fileName, 'index.md')
    const content = fs.readFileSync(elementPath, { encoding: 'utf8' })
    const eventList = extractEventList(content)
    eventList.forEach(({ name, description, reference }) => {
      console.info(`Global Event: ${name}`)
      fs.writeSync(outFile, '[[global-event]]\n')
      fs.writeSync(outFile, `name = "${name}"\n`)
      fs.writeSync(outFile, `description = '''${description}'''\n`)
      fs.writeSync(outFile, `reference = "${reference}"\n`)
      fs.writeSync(outFile, `\n`)
    })
    return
  }
  if (!/^html.+element$/.test(fileName)) return
  const tagName = fileName.slice(4, -7)
  const elementPath = path.join(apiDir, fileName, 'index.md')
  const content = fs.readFileSync(elementPath, { encoding: 'utf8' })
  const eventList = extractEventList(content)
  if (eventList.length === 0) return
  console.info(`Element Event: ${tagName}`)
  elementEventMap[tagName] = eventList
})

// enumerate elements
fs.writeSync(outFile, `\n`)
const elementDir = path.join(mdnDir, 'files/en-us/web/html/element')
const extractElementFromContent = (tagName: string, content: string) => {
  // extract basic information
  let deprecated = false
  let description =
    extractContentBetween(content, '{{HTMLSidebar}}\n\n', '\n\n') ??
    extractContentBetween(content, '{{HTMLSidebar}}{{SeeCompatTable}}\n\n', '\n\n')
  // extractContentBetween(content, '{{HTMLSidebar}}\n\n', '\n\n## ') ??
  // extractContentBetween(content, '{{HTMLSidebar}}\n\n', '\n\n{{EmbedInteractiveExample(') ??
  // extractContentBetween(content, '{{HTMLSidebar}}{{SeeCompatTable}}\n\n', '\n\n## ')
  if (description === null) {
    description =
      extractContentBetween(content, '{{HTMLSidebar}}{{deprecated_header}}\n\n', '\n\n') ??
      extractContentBetween(content, '{{HTMLSidebar}}{{Deprecated_header}}\n\n', '\n\n') ??
      extractContentBetween(content, '{{HTMLSidebar}}{{Deprecated_Header}}\n\n', '\n\n')
    if (description === null) {
      console.error(`Cannot find description for tag "${tagName}". Skipped this tag.`)
      return
    }
    deprecated = true
  }
  const reference = `https://developer.mozilla.org/en-US/docs/Web/HTML/Element/${tagName}`
  console.info(`Element: ${tagName}`)
  fs.writeSync(outFile, '\n[[element]]\n')
  fs.writeSync(outFile, `tag-name = "${tagName}"\n`)
  fs.writeSync(outFile, `description = '''${description}'''\n`)
  fs.writeSync(outFile, `reference = "${reference}"\n`)
  if (deprecated) {
    fs.writeSync(outFile, `deprecated = true\n`)
  }
  fs.writeSync(outFile, `\n`)

  // extract attributes
  const attrsSection = extractContentBetween(content, '## Attributes\n\n', '\n\n## ')
  if (attrsSection !== null) {
    const reference = `https://developer.mozilla.org/en-US/docs/Web/HTML/Element/${tagName}#attributes`
    const attrSegs = extractCascadeLists(attrsSection)
    attrSegs.forEach((seg) => {
      const attrName = extractContentBetween(seg.content, '`', '`')
      if (attrName === null) {
        console.error(`Cannot find a proper attribute in line "- ${seg.content}".`)
        return
      }
      const attrDescLine = seg.children[0]
      if (!attrDescLine || !attrDescLine.content.startsWith(': ')) {
        console.error(
          `Cannot find a proper attribute content for attribute "${attrName}". Skipped this attribute.`,
        )
        return
      }
      let attrDescEnd = attrDescLine.content.lastIndexOf('.')
      if (attrDescEnd < 0) {
        attrDescEnd = attrDescLine.content.length
      }
      const description = attrDescLine.content.slice(2, attrDescEnd + 1)
      fs.writeSync(outFile, '[[element.attribute]]\n')
      fs.writeSync(outFile, `name = "${attrName}"\n`)
      if (description) fs.writeSync(outFile, `description = '''${description}'''\n`)
      fs.writeSync(outFile, `reference = "${reference}"\n`)
      fs.writeSync(outFile, `\n`)
    })
  }

  // write extracted events
  const evList = elementEventMap[tagName]
  evList?.forEach(({ name, description, reference }) => {
    console.info(`Element Event: ${name}`)
    fs.writeSync(outFile, '[[element.event]]\n')
    fs.writeSync(outFile, `name = "${name}"\n`)
    fs.writeSync(outFile, `description = '''${description}'''\n`)
    fs.writeSync(outFile, `reference = "${reference}"\n`)
    fs.writeSync(outFile, `\n`)
  })
}
fs.readdirSync(elementDir).forEach((tagName) => {
  if (tagName.indexOf('.') >= 0) return
  const elementPath = path.join(elementDir, tagName, 'index.md')
  const content = fs.readFileSync(elementPath, { encoding: 'utf8' })
  if (tagName === 'heading_elements') {
    extractElementFromContent('h1', content)
    extractElementFromContent('h2', content)
    extractElementFromContent('h3', content)
    extractElementFromContent('h4', content)
    extractElementFromContent('h5', content)
    extractElementFromContent('h6', content)
  } else {
    extractElementFromContent(tagName, content)
  }
})

// finish
fs.closeSync(outFile)
console.info('Done!')
