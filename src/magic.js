var MAGIC = (function (tome) {
  var halfTurn = Math.PI, fullTurn = 2 * Math.PI;
  function etch(quantity) { return +quantity.toFixed(3); }
  function divine(incantation) { return incantation.split(' ').map(Number); }

  function orb(cx, cy, radius, glow) {
    return "<circle cx='" + etch(cx) + "' cy='" + etch(cy) + "' r='" + etch(radius) + "' opacity='" + glow + "'/>";
  }
  function star(cx, cy, radius, points, stride, turning) {
    for (var vertices = [], vertex = 0; vertex <= points; vertex++) {
      var bearing = turning + vertex * stride % points * fullTurn / points;
      vertices.push(etch(cx + radius * Math.cos(bearing)) + ',' + etch(cy + radius * Math.sin(bearing)));
    }
    return "<polyline points='" + vertices.join(' ') + "'/>";
  }
  // A point on the cubic ley line and its unit heading at moment `fate` (0..1).
  function scry(anchors, fate) {
    var ebb = 1 - fate, place = [0, 0], heading = [0, 0];
    for (var axis = 0; axis < 2; axis++) {
      place[axis] = ebb * ebb * ebb * anchors[0][axis] + 3 * ebb * ebb * fate * anchors[1][axis] + 3 * ebb * fate * fate * anchors[2][axis] + fate * fate * fate * anchors[3][axis];
      heading[axis] = 3 * ebb * ebb * (anchors[1][axis] - anchors[0][axis]) + 6 * ebb * fate * (anchors[2][axis] - anchors[1][axis]) + 3 * fate * fate * (anchors[3][axis] - anchors[2][axis]);
    }
    var reach = Math.max(Math.hypot(heading[0], heading[1]), 1e-4);
    return [place, [heading[0] / reach, heading[1] / reach]];
  }
  function compass(heading) { return Math.atan2(heading[1], heading[0]) * 180 / halfTurn; }

  // B: a braided invocation channel.
  function weaveBraid(spell) {
    var words = spell.split(' '), ink = words[0], parchment = words[1], turns = +words[5], sway = +words[6];
    var numbers = words.slice(7).map(Number);
    var anchors = [[numbers[0], numbers[1]], [numbers[2], numbers[3]], [numbers[4], numbers[5]], [numbers[6], numbers[7]]];
    var knotSizes = numbers.slice(8);
    var leyLine = 'M ' + numbers.slice(0, 2).join(' ') + ' C ' + numbers.slice(2, 4).join(' ') + ', ' + numbers.slice(4, 6).join(' ') + ', ' + numbers.slice(6, 8).join(' ');
    var scroll = "<g data-channel='" + words[2] + "' data-from='" + words[3] + "' data-to='" + words[4] + "' fill='none' stroke='" + ink + "' stroke-linecap='round' stroke-linejoin='round'><path d='" + leyLine + "' stroke-width='8' opacity='.055'/><path d='" + leyLine + "' stroke-width='.65' stroke-dasharray='1 9' opacity='.42'/>";
    [-1, 1].forEach(function (strand) {
      var trail = '';
      for (var step = 0; step <= 96; step++) {
        var fate = step / 96, omen = scry(anchors, fate);
        var drift = strand * sway * Math.sin(fate * turns * fullTurn) * Math.sqrt(Math.max(Math.sin(halfTurn * fate), 0));
        trail += (step ? 'L' : 'M') + etch(omen[0][0] - omen[1][1] * drift) + ' ' + etch(omen[0][1] + omen[1][0] * drift);
      }
      scroll += "<path d='" + trail + "' stroke-width='1.05' opacity='.62'/>";
    });
    knotSizes.forEach(function (knot, nthKnot) {
      var omen = scry(anchors, (nthKnot + 1) / (knotSizes.length + 1)), cross = etch(knot * .45);
      scroll += "<g transform='translate(" + etch(omen[0][0]) + ' ' + etch(omen[0][1]) + ') rotate(' + etch(compass(omen[1])) + ")' fill='" + parchment + "' stroke-width='.8' opacity='.85'>" + star(0, 0, knot, 4, 1, 0) + (nthKnot % 2 ? '' : orb(0, 0, knot * 1.35, .7)) + "<path d='M " + -cross + ' 0 L ' + cross + ' 0 M 0 ' + -cross + ' L 0 ' + cross + "'/></g>";
    });
    var wellspring = scry(anchors, 0), sanctum = scry(anchors, 1);
    return scroll + "<g fill='" + parchment + "' stroke-width='.9'>" + orb(wellspring[0][0], wellspring[0][1], 4, .75) + orb(wellspring[0][0], wellspring[0][1], 1.6, .7) +
      "</g><g transform='translate(" + etch(sanctum[0][0]) + ' ' + etch(sanctum[0][1]) + ') rotate(' + etch(compass(sanctum[1])) + ")' stroke-width='1' opacity='.92'><g fill='" + parchment + "'>" +
      orb(0, 0, 6.5, .9) + orb(0, 0, 3.7, .75) + orb(0, 0, 1.6, .9) + '</g></g></g>';
  }

  // W: a ward ring, its three circles and its language's repeated runes.
  function raiseWardRing(spell, wardRunes) {
    var words = spell.split(' '), cx = +words[0], cy = +words[1], radius = +words[2], runes = wardRunes[words[4]];
    var scroll = "<g stroke-width='" + Math.max(radius * .004, .85).toFixed(3) + "'>" + orb(cx, cy, radius, .92) + orb(cx, cy, radius * .93, .65) + orb(cx, cy, radius * .73, .72);
    for (var repetition = 0; repetition < runes[1]; repetition++) {
      scroll += "<use href='#" + runes[0] + "' transform='translate(" + cx + ' ' + cy + ') rotate(' + etch(+words[3] + repetition * 360 / runes[1]) + ') scale(' + radius + ")'/>";
    }
    return scroll + '</g>';
  }

  // I: an inscription band; each rune cell's weight follows from its glyph proportions.
  function inscribeBand(spell, glyphAtlas) {
    var verses = spell.split(','), circle = divine(verses[0]);
    var cx = circle[0], cy = circle[1], radius = circle[2], inner = circle[3], outer = circle[4];
    var middle = radius * (inner + outer) / 2, bandHeight = radius * (outer - inner), targetHeight = bandHeight * .81;
    var totalWeight = 0, arcWalked = 0;
    var scroll = "<g transform='translate(" + cx + ' ' + cy + ') rotate(' + circle[6] + ")' opacity='.94'>";
    var runeCells = verses.slice(1).map(function (verse, nthCell) {
      var rune = divine(verse), stature = .9 + rune[0] / 500, glyphIds = rune.slice(2);
      var breadth = Math.max.apply(0, glyphIds.map(function (glyphId) { return glyphAtlas[glyphId][2] / glyphAtlas[glyphId][3]; }));
      totalWeight += Math.min(Math.max(breadth, .24), 1.9) * stature * (glyphIds.length > 1 ? .46 : 1) + .19 + (circle[5] == 2 && nthCell % 4 == 0 ? .4 : 0);
      return [totalWeight, stature, rune[1] / 5, glyphIds];
    });
    var weightBefore = 0;
    runeCells.forEach(function (runeCell) {
      var sector = (runeCell[0] - weightBefore) * fullTurn / totalWeight;
      var height = Math.min(targetHeight * runeCell[1], bandHeight * .86);
      var width = Math.min(2 * radius * inner * Math.sin(sector / 2) * .84, 2 * Math.sqrt(Math.max((radius * outer) * (radius * outer) - (middle + height / 2) * (middle + height / 2), 0)) * .97);
      var lean = runeCell[2], leanTurn = lean * halfTurn / 180, cosine = Math.abs(Math.cos(leanTurn)), sine = Math.abs(Math.sin(leanTurn)), twinned = runeCell[3].length > 1;
      weightBefore = runeCell[0];
      scroll += "<g transform='rotate(" + etch((arcWalked + sector / 2) * 180 / halfTurn) + ') translate(0 ' + -etch(middle) + ")'>";
      runeCell[3].forEach(function (glyphId, nthGlyph) {
        var measure = glyphAtlas[glyphId], leaningWidth = measure[2] * cosine + measure[3] * sine, leaningHeight = measure[3] * cosine + measure[2] * sine;
        var glyphHeight = twinned ? height * .44 : height, enchantment = Math.min(width / leaningWidth, glyphHeight / leaningHeight);
        scroll += "<use href='#glyph-" + glyphId + "' transform='translate(0 " + (twinned ? etch(height * (nthGlyph ? .27 : -.27)) : 0) + ') rotate(' + lean + ') scale(' + enchantment + ' ' + -enchantment + ') translate(' + -measure[0] + ' ' + -measure[1] + ")'/>";
      });
      scroll += '</g>';
      arcWalked += sector;
    });
    return scroll + '</g>';
  }

  // N: a reversible name contour, one radial vertex per nibble.
  function traceNameSigil(spell) {
    var words = spell.split(' '), nibbles = words[4], vertices = [];
    for (var nthNibble = 0; nthNibble < nibbles.length; nthNibble++) {
      var bearing = -halfTurn / 2 + nthNibble * fullTurn / nibbles.length, reach = .72 + parseInt(nibbles[nthNibble], 16) * .014;
      vertices.push(etch(reach * Math.cos(bearing) * 1e3) / 1e3 + ',' + etch(reach * Math.sin(bearing) * 1e3) / 1e3);
    }
    return "<polygon data-name-sigil='1' data-entity='" + words[0] + "' points='" + vertices.join(' ') + "' transform='translate(" + words[1] + ' ' + words[2] + ') scale(' + words[3] * .52 + ")' stroke-width='.007' opacity='.65' stroke-linejoin='round'/>";
  }

  // Y: a regular star polygon.
  function drawStar(spell) {
    var words = divine(spell), vertices = [];
    for (var vertex = 0; vertex <= words[3]; vertex++) {
      var bearing = words[5] + vertex * words[4] % words[3] * fullTurn / words[3];
      vertices.push((words[0] + words[2] * Math.cos(bearing)).toFixed(5) + ',' + (words[1] + words[2] * Math.sin(bearing)).toFixed(5));
    }
    return "<polyline points='" + vertices.join(' ') + "'/>";
  }

  // Keys are the macro element names written into compact magic SVG.
  var spellbook = { B: weaveBraid, W: raiseWardRing, I: inscribeBand, N: traceNameSigil, Y: drawStar };
  if (tome) {
    var glyphAtlas = {}, wardRunes = {}, unfolding = '';
    tome.querySelectorAll('defs>path[k]').forEach(function (glyphPath) { glyphAtlas[glyphPath.id.slice(6)] = divine(glyphPath.getAttribute('k')); });
    tome.querySelectorAll('defs>g[l]').forEach(function (runeTemplate) { wardRunes[runeTemplate.getAttribute('l')] = [runeTemplate.id, +runeTemplate.getAttribute('n')]; });
    var sealedSpells = [].slice.call(tome.querySelectorAll('B,W,I,N,Y'));
    sealedSpells.forEach(function (sealed) {
      unfolding += '<g>' + spellbook[sealed.localName](sealed.getAttribute('d'), sealed.localName == 'I' ? glyphAtlas : wardRunes) + '</g>';
    });
    var revealed = new DOMParser().parseFromString("<svg xmlns='http://www.w3.org/2000/svg'>" + unfolding + '</svg>', 'image/svg+xml').documentElement.children;
    sealedSpells.forEach(function (sealed, nthSpell) { sealed.parentNode.replaceChild(tome.importNode(revealed[nthSpell], true), sealed); });
  }
  return spellbook;
})(typeof document == 'object' && document);
