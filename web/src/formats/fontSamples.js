// What a font is shown with, in its viewer and its thumbnail: lines in the script it is made for, using only the characters it has.

/** Three lines for a thumbnail: capitals, small letters and digits, or what a script has in their place. */
export const LINES = {
  Latin: ['ABCDEFG', 'abcdefg', '0123456'],
  Greek: ['ΑΒΓΔΕΖΗ', 'αβγδεζη', '0123456'],
  Cyrillic: ['АБВГДЕЖ', 'абвгдеж', '0123456'],
  Armenian: ['ԱԲԳԴԵԶԷ', 'աբգդեզէ', '0123456'],
  Georgian: ['აბგდევზ', 'თიკლმნო', '0123456'],
  Hebrew: ['אבגדהוז', 'חטיכלמנ', '0123456'],
  Arabic: ['أبجد هوز', 'حطي كلمن', '٠١٢٣٤٥٦'],
  Thai: ['กขคงจฉช', 'ซฌญฎฏฐฑ', '๐๑๒๓๔๕๖'],
  Lao: ['ກຂຄງຈຊຍ', 'ດຕຖທນບປ', '໐໑໒໓໔໕໖'],
  Khmer: ['កខគឃងចឆ', 'ជឈញដឋឌឍ', '០១២៣៤៥៦'],
  Myanmar: ['ကခဂဃငစဆ', 'ဇဈညဋဌဍဎ', '၀၁၂၃၄၅၆'],
  Tibetan: ['ཀཁགངཅཆཇ', 'ཉཏཐདནཔཕ', '༠༡༢༣༤༥༦'],
  Ethiopic: ['ሀለሐመሠረሰ', 'ሸቀበተቸኀነ', '0123456'],
  Devanagari: ['अआइईउऊए', 'कखगघङचछ', '०१२३४५६'],
  Bengali: ['অআইঈউঊএ', 'কখগঘঙচছ', '০১২৩৪৫৬'],
  Gurmukhi: ['ਅਆਇਈਉਊਏ', 'ਕਖਗਘਙਚਛ', '੦੧੨੩੪੫੬'],
  Gujarati: ['અઆઇઈઉઊએ', 'કખગઘઙચછ', '૦૧૨૩૪૫૬'],
  Tamil: ['அஆஇஈஉஊஎ', 'கஙசஞடணத', '௦௧௨௩௪௫௬'],
  Telugu: ['అఆఇఈఉఊఎ', 'కఖగఘఙచఛ', '౦౧౨౩౪౫౬'],
  Kannada: ['ಅಆಇಈಉಊಎ', 'ಕಖಗಘಙಚಛ', '೦೧೨೩೪೫೬'],
  Malayalam: ['അആഇഈഉഊഎ', 'കഖഗഘങചഛ', '൦൧൨൩൪൫൬'],
  Sinhala: ['අආඇඈඉඊඋ', 'කඛගඝඞචඡ', '0123456'],
  Chinese: ['永字八法', '天地玄黄', '0123456'],
  Japanese: ['あいうえお', 'アイウエオ', '永字八法'],
  Korean: ['가나다라마', '바사아자차', '0123456'],
}

/** Scripts written right to left. */
export const RTL = new Set(['Arabic', 'Hebrew'])

/** Whether the font maps a code point, by the ranges its description lists (`[[first, last], …]`, sorted). */
export function has(ranges, cp) {
  let lo = 0
  let hi = ranges.length - 1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    const [a, b] = ranges[mid]
    if (cp < a) hi = mid - 1
    else if (cp > b) lo = mid + 1
    else return true
  }
  return false
}

const BLANK = /\s/u

/** The text with the characters the font lacks left out; spaces stay. */
export function keep(ranges, text) {
  return [...text].filter((ch) => BLANK.test(ch) || has(ranges, ch.codePointAt(0))).join('').replace(/\s+/gu, ' ').trim()
}

/** The first characters the font has that show something: a symbol font's sample. */
export function firstChars(ranges, count) {
  const out = []
  for (const [a, b] of ranges) {
    for (let cp = a; cp <= b && out.length < count; cp++) {
      const ch = String.fromCodePoint(cp)
      if (!BLANK.test(ch) && !/\p{M}/u.test(ch)) out.push(ch)
    }
    if (out.length >= count) break
  }
  return out
}

/** The thumbnail's lines: its main script's, else a grid of its first characters. */
export function thumbLines(info) {
  const lines = (LINES[info.primary] || []).map((l) => keep(info.ranges, l)).filter(Boolean)
  if (lines.length) return lines
  const chars = firstChars(info.ranges, 12)
  const out = []
  for (let i = 0; i < chars.length; i += 4) out.push(chars.slice(i, i + 4).join(' '))
  return out
}

/** The viewer's sample: a short title, a line with every letter where the script has one, and a paragraph. Scripts without their own use their thumbnail lines. */
export const TEXTS = {
  Latin: {
    title: 'Handgloves',
    line: 'The quick brown fox jumps over the lazy dog. 0123456789',
    body: 'Type is a quiet art. A good face carries a long text without drawing attention to itself, yet gives a headline a voice of its own. Letters that sit well together at small sizes and keep their character when set large make a font worth keeping.',
  },
  Cyrillic: { title: 'Шрифт', line: 'Съешь же ещё этих мягких французских булок, да выпей чаю.', body: 'Широкая электрификация южных губерний даст мощный толчок подъёму сельского хозяйства.' },
  Greek: { title: 'Γραμματοσειρά', line: 'Ξεσκεπάζω την ψυχοφθόρα βδελυγμία.', body: 'Τάχιστη αλώπηξ βαφής ψημένη γη, δρασκελίζει υπέρ νωθρού κυνός.' },
  Hebrew: { title: 'גופן', line: 'דג סקרן שט בים מאוכזב ולפתע מצא חברה', body: 'עטלף אבק נס דרך מזגן שהתפוצץ כי חם' },
  Arabic: { title: 'خط', line: 'صِف خَلقَ خَودِ كَمِثلِ الشَمسِ إِذ بَزَغَت', body: 'نص حكيم له سر قاطع وذو شأن عظيم مكتوب على ثوب أخضر ومغلف بجلد أزرق' },
  Thai: { title: 'แบบอักษร', line: 'เป็นมนุษย์สุดประเสริฐเลิศคุณค่า', body: 'กว่าบรรดาฝูงสัตว์เดรัจฉาน จงฝ่าฟันพัฒนาวิชาการ อย่าล้างผลาญฤๅเข่นฆ่าบีฑาใคร' },
  Devanagari: { title: 'देवनागरी', line: 'ऋषियों को सताने वाले दुष्ट राक्षसों के राजा रावण का सर्वनाश करने वाले', body: 'विष्णुवतार भगवान श्रीराम, अयोध्या के महाराज दशरथ के बड़े सपुत्र थे।' },
  Chinese: { title: '永字八法', line: '天地玄黄，宇宙洪荒。日月盈昃，辰宿列张。', body: '寒来暑往，秋收冬藏。闰余成岁，律吕调阳。云腾致雨，露结为霜。金生丽水，玉出昆冈。' },
  Japanese: { title: '永字八法', line: 'いろはにほへと ちりぬるを わかよたれそ つねならむ', body: 'うゐのおくやま けふこえて あさきゆめみし ゑひもせす　天地玄黄、宇宙洪荒。' },
  Korean: { title: '한글', line: '키스의 고유조건은 입술끼리 만나야 하고 특별한 기술은 필요치 않다.', body: '다람쥐 헌 쳇바퀴에 타고파. 정 참판 양반댁 규수 큰 교자 타고 혼례 치른 날.' },
}

/** The viewer's sample for a font: its script's own, else its thumbnail lines. */
export function sampleFor(info) {
  const t = TEXTS[info.primary]
  if (t) return t
  const lines = thumbLines(info)
  return { title: lines[0] || '', line: lines.join(' '), body: lines.join(' ') }
}

/** Every character the font maps, in order, from its ranges. */
export function charList(ranges) {
  const out = []
  for (const [a, b] of ranges) for (let cp = a; cp <= b; cp++) out.push(cp)
  return out
}

/** A code point as Unicode writes it: U+0041, U+1F600. */
export const codePoint = (cp) => 'U+' + cp.toString(16).toUpperCase().padStart(4, '0')
