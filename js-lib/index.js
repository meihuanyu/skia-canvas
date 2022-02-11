"use strict";

require("core-js/modules/es6.symbol.js");

require("core-js/modules/web.dom.iterable.js");

require("core-js/modules/es6.regexp.match.js");

require("core-js/modules/es6.regexp.to-string.js");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) { symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); } keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { _defineProperty(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

function _defineProperty(obj, key, value) { if (key in obj) { Object.defineProperty(obj, key, { value: value, enumerable: true, configurable: true, writable: true }); } else { obj[key] = value; } return obj; }

const fs = require('fs'),
      {
  EventEmitter
} = require('events'),
      {
  inspect
} = require('util'),
      {
  sync: glob,
  hasMagic
} = require('glob'),
      get = require('simple-get'),
      geometry = require('./geometry'),
      css = require('./css'),
      io = require('./io'),
      REPR = inspect.custom; //
// Neon <-> Node interface
//


const ø = Symbol.for('📦'),
      // the attr containing the boxed struct
core = obj => (obj || {})[ø],
      // dereference the boxed struct
wrap = (type, struct) => {
  // create new instance for struct
  let obj = internal(Object.create(type.prototype), ø, struct);
  return struct && internal(obj, 'native', neon[type.name]);
},
      neon = Object.entries(require('./v6')).reduce((api, _ref) => {
  let [name, fn] = _ref;
  let [_, struct, getset, attr] = name.match(/(.*?)_(?:([sg]et)_)?(.*)/),
      cls = api[struct] || (api[struct] = {}),
      slot = getset ? cls[attr] || (cls[attr] = {}) : cls;
  slot[getset || attr] = fn;
  return api;
}, {});

class RustClass {
  constructor(type) {
    internal(this, 'native', neon[type.name]);
  }

  alloc() {
    for (var _len = arguments.length, args = new Array(_len), _key = 0; _key < _len; _key++) {
      args[_key] = arguments[_key];
    }

    return this.init('new', ...args);
  }

  init(fn) {
    for (var _len2 = arguments.length, args = new Array(_len2 > 1 ? _len2 - 1 : 0), _key2 = 1; _key2 < _len2; _key2++) {
      args[_key2 - 1] = arguments[_key2];
    }

    return internal(this, ø, this.native[fn](null, ...args));
  }

  ref(key, val) {
    return arguments.length > 1 ? this[Symbol.for(key)] = val : this[Symbol.for(key)];
  }

  prop(attr, val) {
    let getset = arguments.length > 1 ? 'set' : 'get';
    return this.native[attr][getset](this[ø], val);
  }

  ƒ(fn) {
    for (var _len3 = arguments.length, args = new Array(_len3 > 1 ? _len3 - 1 : 0), _key3 = 1; _key3 < _len3; _key3++) {
      args[_key3 - 1] = arguments[_key3];
    }

    return this.native[fn](this[ø], ...args);
  }

} // shorthands for attaching read-only attributes


const readOnly = (obj, attr, value) => Object.defineProperty(obj, attr, {
  value,
  writable: false,
  enumerable: true
});

const internal = (obj, attr, value) => Object.defineProperty(obj, attr, {
  value,
  writable: false,
  enumerable: false
}); // convert arguments list to a string of type abbreviations


function signature(args) {
  return args.map(v => Array.isArray(v) ? 'a' : {
    string: 's',
    number: 'n',
    object: 'o'
  }[typeof v] || 'x').join('');
}

const toString = val => typeof val == 'string' ? val : new String(val).toString(); //
// Helpers to reconcile Skia and DOMMatrix’s disagreement about row/col orientation
//


function toSkMatrix(jsMatrix) {
  if (Array.isArray(jsMatrix) && jsMatrix.length == 6) {
    var [a, b, c, d, e, f, m14, m24, m44] = jsMatrix.concat(0, 0, 1);
  } else if (jsMatrix instanceof geometry.DOMMatrix) {
    var {
      a,
      b,
      c,
      d,
      e,
      f,
      m14,
      m24,
      m44
    } = jsMatrix;
  }

  return [a, c, e, b, d, f, m14, m24, m44];
}

function fromSkMatrix(skMatrix) {
  let [a, b, c, d, e, f, p0, p1, p2] = skMatrix;
  return new geometry.DOMMatrix([a, d, 0, p0, b, e, 0, p1, 0, 0, 1, 0, c, f, 0, p2]);
} //
// The Canvas API
//


class Canvas extends RustClass {
  constructor(width, height) {
    super(Canvas).alloc();
    Canvas.contexts.set(this, []);
    Object.assign(this, {
      width,
      height
    });
  }

  getContext(kind) {
    return kind == "2d" ? Canvas.contexts.get(this)[0] || this.newPage() : null;
  }

  get width() {
    return ~~this.prop('width');
  }

  set width(w) {
    this.prop('width', typeof w == 'number' && !Number.isNaN(w) && w >= 0 ? w : 300);
    if (Canvas.contexts.get(this)[0]) this.getContext("2d").ƒ('resetSize', core(this));
  }

  get height() {
    return ~~this.prop('height');
  }

  set height(h) {
    this.prop('height', h = typeof h == 'number' && !Number.isNaN(h) && h >= 0 ? h : 150);
    if (Canvas.contexts.get(this)[0]) this.getContext("2d").ƒ('resetSize', core(this));
  }

  newPage(width, height) {
    let ctx = new CanvasRenderingContext2D(core(this));
    Canvas.parent.set(ctx, this);
    Canvas.contexts.get(this).unshift(ctx);

    if (arguments.length == 2) {
      Object.assign(this, {
        width,
        height
      });
    }

    return ctx;
  }

  get pages() {
    return Canvas.contexts.get(this).slice().reverse();
  }

  get png() {
    return this.toBuffer("png");
  }

  get jpg() {
    return this.toBuffer("jpg");
  }

  get pdf() {
    return this.toBuffer("pdf");
  }

  get svg() {
    return this.toBuffer("svg");
  }

  get async() {
    return this.prop('async');
  }

  set async(flag) {
    if (!flag) {
      process.emitWarning("Use the saveAsSync, toBufferSync, and toDataURLSync methods instead of setting the Canvas `async` property to false", "DeprecationWarning");
    }

    this.prop('async', flag);
  }

  saveAs(filename) {
    let opts = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
    if (!this.async) return this.saveAsSync(...arguments); // support while deprecated

    opts = typeof opts == 'number' ? {
      quality: opts
    } : opts;
    let {
      format,
      quality,
      pages,
      padding,
      pattern,
      density,
      outline,
      matte
    } = io.options(this.pages, _objectSpread({
      filename
    }, opts)),
        args = [pages.map(core), pattern, padding, format, quality, density, outline, matte],
        worker = new EventEmitter();
    this.ƒ("save", (result, msg) => worker.emit(result, msg), ...args);
    return new Promise((res, rej) => worker.once('ok', res).once('err', msg => rej(new Error(msg))));
  }

  saveAsSync(filename) {
    let opts = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
    opts = typeof opts == 'number' ? {
      quality: opts
    } : opts;
    let {
      format,
      quality,
      pages,
      padding,
      pattern,
      density,
      outline,
      matte
    } = io.options(this.pages, _objectSpread({
      filename
    }, opts));
    this.ƒ("saveSync", pages.map(core), pattern, padding, format, quality, density, outline, matte);
  }

  toBuffer() {
    let extension = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : "png";
    let opts = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
    if (!this.async) return this.toBufferSync(...arguments); // support while deprecated

    opts = typeof opts == 'number' ? {
      quality: opts
    } : opts;
    let {
      format,
      quality,
      pages,
      density,
      outline,
      matte
    } = io.options(this.pages, _objectSpread({
      extension
    }, opts)),
        args = [pages.map(core), format, quality, density, outline, matte],
        worker = new EventEmitter();
    this.ƒ("toBuffer", (result, msg) => worker.emit(result, msg), ...args);
    return new Promise((res, rej) => worker.once('ok', res).once('err', msg => rej(new Error(msg))));
  }

  toBufferSync() {
    let extension = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : "png";
    let opts = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
    opts = typeof opts == 'number' ? {
      quality: opts
    } : opts;
    let {
      format,
      quality,
      pages,
      density,
      outline,
      matte
    } = io.options(this.pages, _objectSpread({
      extension
    }, opts));
    return this.ƒ("toBufferSync", pages.map(core), format, quality, density, outline, matte);
  }

  toDataURL() {
    let extension = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : "png";
    let opts = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
    if (!this.async) return this.toDataURLSync(...arguments); // support while deprecated

    opts = typeof opts == 'number' ? {
      quality: opts
    } : opts;
    let {
      mime
    } = io.options(this.pages, _objectSpread({
      extension
    }, opts)),
        buffer = this.toBuffer(extension, opts);
    return buffer.then(data => "data:".concat(mime, ";base64,").concat(data.toString('base64')));
  }

  toDataURLSync() {
    let extension = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : "png";
    let opts = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
    opts = typeof opts == 'number' ? {
      quality: opts
    } : opts;
    let {
      mime
    } = io.options(this.pages, _objectSpread({
      extension
    }, opts)),
        buffer = this.toBufferSync(extension, opts);
    return "data:".concat(mime, ";base64,").concat(buffer.toString('base64'));
  }

  [REPR](depth, options) {
    let {
      width,
      height,
      async,
      pages
    } = this;
    return "Canvas ".concat(inspect({
      width,
      height,
      async,
      pages
    }, options));
  }

}

_defineProperty(Canvas, "parent", new WeakMap());

_defineProperty(Canvas, "contexts", new WeakMap());

class CanvasGradient extends RustClass {
  constructor(style) {
    super(CanvasGradient);
    style = (style || "").toLowerCase();

    for (var _len4 = arguments.length, coords = new Array(_len4 > 1 ? _len4 - 1 : 0), _key4 = 1; _key4 < _len4; _key4++) {
      coords[_key4 - 1] = arguments[_key4];
    }

    if (['linear', 'radial', 'conic'].includes(style)) this.init(style, ...coords);else throw new Error("Function is not a constructor (use CanvasRenderingContext2D's \"createConicGradient\", \"createLinearGradient\", and \"createRadialGradient\" methods instead)");
  }

  addColorStop(offset, color) {
    if (offset >= 0 && offset <= 1) this.ƒ('addColorStop', offset, color);else throw new Error("Color stop offsets must be between 0.0 and 1.0");
  }

  [REPR](depth, options) {
    return "CanvasGradient (".concat(this.ƒ("repr"), ")");
  }

}

class CanvasPattern extends RustClass {
  constructor(src, repeat) {
    super(CanvasPattern);

    if (src instanceof Image) {
      this.init('from_image', core(src), repeat);
    } else if (src instanceof Canvas) {
      let ctx = src.getContext('2d');
      this.init('from_canvas', core(ctx), repeat);
    } else {
      throw new Error("CanvasPatterns require a source Image or a Canvas");
    }
  }

  setTransform(matrix) {
    if (arguments.length > 1) matrix = [...arguments];
    this.ƒ('setTransform', toSkMatrix(matrix));
  }

  [REPR](depth, options) {
    return "CanvasPattern (".concat(this.ƒ("repr"), ")");
  }

}

class CanvasTexture extends RustClass {
  constructor(spacing) {
    let {
      path,
      line,
      color,
      angle,
      offset = 0
    } = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
    super(CanvasTexture);
    let [x, y] = typeof offset == 'number' ? [offset, offset] : offset.slice(0, 2);
    let [h, v] = typeof spacing == 'number' ? [spacing, spacing] : spacing.slice(0, 2);
    path = core(path);
    line = line != null ? line : path ? 0 : 1;
    angle = angle != null ? angle : path ? 0 : -Math.PI / 4;
    this.alloc(path, color, line, angle, h, v, x, y);
  }

  [REPR](depth, options) {
    return "CanvasTexture (".concat(this.ƒ("repr"), ")");
  }

}

class CanvasRenderingContext2D extends RustClass {
  constructor(canvas) {
    try {
      super(CanvasRenderingContext2D).alloc(canvas);
    } catch (e) {
      throw new TypeError("Function is not a constructor (use Canvas's \"getContext\" method instead)");
    }
  }

  get canvas() {
    return Canvas.parent.get(this);
  } // -- grid state ------------------------------------------------------------


  save() {
    this.ƒ('save');
  }

  restore() {
    this.ƒ('restore');
  }

  get currentTransform() {
    return fromSkMatrix(this.prop('currentTransform'));
  }

  set currentTransform(matrix) {
    this.prop('currentTransform', toSkMatrix(matrix));
  }

  resetTransform() {
    this.ƒ('resetTransform');
  }

  getTransform() {
    return this.currentTransform;
  }

  setTransform(matrix) {
    this.currentTransform = arguments.length > 1 ? [...arguments] : matrix;
  }

  transform() {
    for (var _len5 = arguments.length, terms = new Array(_len5), _key5 = 0; _key5 < _len5; _key5++) {
      terms[_key5] = arguments[_key5];
    }

    this.ƒ('transform', ...terms);
  }

  translate(x, y) {
    this.ƒ('translate', x, y);
  }

  scale(x, y) {
    this.ƒ('scale', x, y);
  }

  rotate(angle) {
    this.ƒ('rotate', angle);
  }

  createProjection(quad, basis) {
    return fromSkMatrix(this.ƒ("createProjection", [quad].flat(), [basis].flat()));
  } // -- bézier paths ----------------------------------------------------------


  beginPath() {
    this.ƒ('beginPath');
  }

  rect(x, y, width, height) {
    this.ƒ('rect', ...arguments);
  }

  arc(x, y, radius, startAngle, endAngle, isCCW) {
    this.ƒ('arc', ...arguments);
  }

  ellipse(x, y, xRadius, yRadius, rotation, startAngle, endAngle, isCCW) {
    this.ƒ('ellipse', ...arguments);
  }

  moveTo(x, y) {
    this.ƒ('moveTo', x, y);
  }

  lineTo(x, y) {
    this.ƒ('lineTo', x, y);
  }

  arcTo(x1, y1, x2, y2, radius) {
    this.ƒ('arcTo', ...arguments);
  }

  bezierCurveTo(cp1x, cp1y, cp2x, cp2y, x, y) {
    this.ƒ('bezierCurveTo', ...arguments);
  }

  quadraticCurveTo(cpx, cpy, x, y) {
    this.ƒ('quadraticCurveTo', ...arguments);
  }

  conicCurveTo(cpx, cpy, x, y, weight) {
    this.ƒ("conicCurveTo", ...arguments);
  }

  closePath() {
    this.ƒ('closePath');
  }

  isPointInPath(x, y) {
    return this.ƒ('isPointInPath', x, y);
  }

  isPointInStroke(x, y) {
    return this.ƒ('isPointInStroke', x, y);
  } // -- using paths -----------------------------------------------------------


  fill(path, rule) {
    if (path instanceof Path2D) this.ƒ('fill', core(path), rule);else this.ƒ('fill', path); // 'path' is the optional winding-rule
  }

  stroke(path, rule) {
    if (path instanceof Path2D) this.ƒ('stroke', core(path), rule);else this.ƒ('stroke', path); // 'path' is the optional winding-rule
  }

  clip(path, rule) {
    if (path instanceof Path2D) this.ƒ('clip', core(path), rule);else this.ƒ('clip', path); // 'path' is the optional winding-rule
  } // -- shaders ---------------------------------------------------------------


  createPattern(image, repetition) {
    return new CanvasPattern(...arguments);
  }

  createLinearGradient(x0, y0, x1, y1) {
    return new CanvasGradient("Linear", ...arguments);
  }

  createRadialGradient(x0, y0, r0, x1, y1, r1) {
    return new CanvasGradient("Radial", ...arguments);
  }

  createConicGradient(startAngle, x, y) {
    return new CanvasGradient("Conic", ...arguments);
  }

  createTexture(spacing, options) {
    return new CanvasTexture(spacing, options);
  } // -- fill & stroke ---------------------------------------------------------


  fillRect(x, y, width, height) {
    this.ƒ('fillRect', ...arguments);
  }

  strokeRect(x, y, width, height) {
    this.ƒ('strokeRect', ...arguments);
  }

  clearRect(x, y, width, height) {
    this.ƒ('clearRect', ...arguments);
  }

  set fillStyle(style) {
    let isShader = style instanceof CanvasPattern || style instanceof CanvasGradient || style instanceof CanvasTexture,
        [ref, val] = isShader ? [style, core(style)] : [null, style];
    this.ref('fill', ref);
    this.prop('fillStyle', val);
  }

  get fillStyle() {
    let style = this.prop('fillStyle');
    return style === null ? this.ref('fill') : style;
  }

  set strokeStyle(style) {
    let isShader = style instanceof CanvasPattern || style instanceof CanvasGradient || style instanceof CanvasTexture,
        [ref, val] = isShader ? [style, core(style)] : [null, style];
    this.ref('stroke', ref);
    this.prop('strokeStyle', val);
  }

  get strokeStyle() {
    let style = this.prop('strokeStyle');
    return style === null ? this.ref('stroke') : style;
  } // -- line style ------------------------------------------------------------


  getLineDash() {
    return this.ƒ("getLineDash");
  }

  setLineDash(segments) {
    this.ƒ("setLineDash", segments);
  }

  get lineCap() {
    return this.prop("lineCap");
  }

  set lineCap(style) {
    this.prop("lineCap", style);
  }

  get lineDashFit() {
    return this.prop("lineDashFit");
  }

  set lineDashFit(style) {
    this.prop("lineDashFit", style);
  }

  get lineDashMarker() {
    return wrap(Path2D, this.prop("lineDashMarker"));
  }

  set lineDashMarker(path) {
    this.prop("lineDashMarker", path instanceof Path2D ? core(path) : path);
  }

  get lineDashOffset() {
    return this.prop("lineDashOffset");
  }

  set lineDashOffset(offset) {
    this.prop("lineDashOffset", offset);
  }

  get lineJoin() {
    return this.prop("lineJoin");
  }

  set lineJoin(style) {
    this.prop("lineJoin", style);
  }

  get lineWidth() {
    return this.prop("lineWidth");
  }

  set lineWidth(width) {
    this.prop("lineWidth", width);
  }

  get miterLimit() {
    return this.prop("miterLimit");
  }

  set miterLimit(limit) {
    this.prop("miterLimit", limit);
  } // -- imagery ---------------------------------------------------------------


  get imageSmoothingEnabled() {
    return this.prop("imageSmoothingEnabled");
  }

  set imageSmoothingEnabled(flag) {
    this.prop("imageSmoothingEnabled", !!flag);
  }

  get imageSmoothingQuality() {
    return this.prop("imageSmoothingQuality");
  }

  set imageSmoothingQuality(level) {
    this.prop("imageSmoothingQuality", level);
  }

  putImageData(imageData) {
    for (var _len6 = arguments.length, coords = new Array(_len6 > 1 ? _len6 - 1 : 0), _key6 = 1; _key6 < _len6; _key6++) {
      coords[_key6 - 1] = arguments[_key6];
    }

    this.ƒ('putImageData', imageData, ...coords);
  }

  createImageData(width, height) {
    return new ImageData(width, height);
  }

  getImageData(x, y, width, height) {
    let w = Math.floor(width),
        h = Math.floor(height),
        buffer = this.ƒ('getImageData', x, y, w, h);
    return new ImageData(buffer, w, h);
  }

  drawImage(image) {
    for (var _len7 = arguments.length, coords = new Array(_len7 > 1 ? _len7 - 1 : 0), _key7 = 1; _key7 < _len7; _key7++) {
      coords[_key7 - 1] = arguments[_key7];
    }

    if (image instanceof Canvas) {
      this.ƒ('drawCanvas', core(image.getContext('2d')), ...coords);
    } else if (image instanceof Image) {
      this.ƒ('drawRaster', core(image), ...coords);
    } else {
      throw new Error("Expected an Image or a Canvas argument");
    }
  } // -- typography ------------------------------------------------------------


  get font() {
    return this.prop('font');
  }

  set font(str) {
    this.prop('font', css.font(str));
  }

  get textAlign() {
    return this.prop("textAlign");
  }

  set textAlign(mode) {
    this.prop("textAlign", mode);
  }

  get textBaseline() {
    return this.prop("textBaseline");
  }

  set textBaseline(mode) {
    this.prop("textBaseline", mode);
  }

  get direction() {
    return this.prop("direction");
  }

  set direction(mode) {
    this.prop("direction", mode);
  }

  measureText(text, maxWidth) {
    text = this.textWrap ? text : text + '\u200b'; // include trailing whitespace by default

    let [metrics, ...lines] = this.ƒ('measureText', toString(text), maxWidth);
    return new TextMetrics(metrics, lines);
  }

  fillText(text, x, y, maxWidth) {
    this.ƒ('fillText', toString(text), x, y, maxWidth);
  }

  strokeText(text, x, y, maxWidth) {
    this.ƒ('strokeText', toString(text), x, y, maxWidth);
  }

  outlineText(text) {
    let path = this.ƒ('outlineText', toString(text));
    return path ? wrap(Path2D, path) : null;
  } // -- non-standard typography extensions --------------------------------------------


  get fontVariant() {
    return this.prop('fontVariant');
  }

  set fontVariant(str) {
    this.prop('fontVariant', css.variant(str));
  }

  get textTracking() {
    return this.prop("textTracking");
  }

  set textTracking(ems) {
    this.prop("textTracking", ems);
  }

  get textWrap() {
    return this.prop("textWrap");
  }

  set textWrap(flag) {
    this.prop("textWrap", !!flag);
  } // -- effects ---------------------------------------------------------------


  get globalCompositeOperation() {
    return this.prop("globalCompositeOperation");
  }

  set globalCompositeOperation(blend) {
    this.prop("globalCompositeOperation", blend);
  }

  get globalAlpha() {
    return this.prop("globalAlpha");
  }

  set globalAlpha(alpha) {
    this.prop("globalAlpha", alpha);
  }

  get shadowBlur() {
    return this.prop("shadowBlur");
  }

  set shadowBlur(level) {
    this.prop("shadowBlur", level);
  }

  get shadowColor() {
    return this.prop("shadowColor");
  }

  set shadowColor(color) {
    this.prop("shadowColor", color);
  }

  get shadowOffsetX() {
    return this.prop("shadowOffsetX");
  }

  set shadowOffsetX(x) {
    this.prop("shadowOffsetX", x);
  }

  get shadowOffsetY() {
    return this.prop("shadowOffsetY");
  }

  set shadowOffsetY(y) {
    this.prop("shadowOffsetY", y);
  }

  get filter() {
    return this.prop('filter');
  }

  set filter(str) {
    this.prop('filter', css.filter(str));
  }

  [REPR](depth, options) {
    let props = ["canvas", "currentTransform", "fillStyle", "strokeStyle", "font", "fontVariant", "direction", "textAlign", "textBaseline", "textTracking", "textWrap", "globalAlpha", "globalCompositeOperation", "imageSmoothingEnabled", "imageSmoothingQuality", "filter", "shadowBlur", "shadowColor", "shadowOffsetX", "shadowOffsetY", "lineCap", "lineDashOffset", "lineJoin", "lineWidth", "miterLimit"];
    let info = {};

    if (depth > 0) {
      for (var prop of props) {
        try {
          info[prop] = this[prop];
        } catch (_unused) {
          info[prop] = undefined;
        }
      }
    }

    return "CanvasRenderingContext2D ".concat(inspect(info, options));
  }

}

const _expand = paths => [paths].flat(2).map(pth => hasMagic(pth) ? glob(pth) : pth).flat();

class FontLibrary extends RustClass {
  constructor() {
    super(FontLibrary);
  }

  get families() {
    return this.prop('families');
  }

  has(familyName) {
    return this.ƒ('has', familyName);
  }

  family(name) {
    return this.ƒ('family', name);
  }

  use() {
    for (var _len8 = arguments.length, args = new Array(_len8), _key8 = 0; _key8 < _len8; _key8++) {
      args[_key8] = arguments[_key8];
    }

    let sig = signature(args);

    if (sig == 'o') {
      let results = {};

      for (let [alias, paths] of Object.entries(args.shift())) {
        results[alias] = this.ƒ("addFamily", alias, _expand(paths));
      }

      return results;
    } else if (sig.match(/^s?[as]$/)) {
      let fonts = _expand(args.pop());

      let alias = args.shift();
      return this.ƒ("addFamily", alias, fonts);
    } else {
      throw new Error("Expected an array of file paths or an object mapping family names to font files");
    }
  }

}

class Image extends RustClass {
  constructor() {
    super(Image).alloc();
  }

  get complete() {
    return this.prop('complete');
  }

  get height() {
    return this.prop('height');
  }

  get width() {
    return this.prop('width');
  }

  get src() {
    return this.prop('src');
  }

  set src(src) {
    var noop = () => {},
        onload = img => fetch.emit('ok', img),
        onerror = err => fetch.emit('err', err),
        passthrough = fn => arg => {
      (fn || noop)(arg);
      delete this._fetch;
    },
        data;

    if (this._fetch) this._fetch.removeAllListeners();
    let fetch = this._fetch = new EventEmitter().once('ok', passthrough(this.onload)).once('err', passthrough(this.onerror));

    if (Buffer.isBuffer(src)) {
      [data, src] = [src, ''];
    } else if (typeof src != 'string') {
      return;
    } else if (/^\s*data:/.test(src)) {
      // data URI
      let split = src.indexOf(','),
          enc = src.lastIndexOf('base64', split) !== -1 ? 'base64' : 'utf8',
          content = src.slice(split + 1);
      data = Buffer.from(content, enc);
    } else if (/^\s*https?:\/\//.test(src)) {
      // remote URL
      get.concat(src, (err, res, data) => {
        let code = (res || {}).statusCode;
        if (err) onerror(err);else if (code < 200 || code >= 300) {
          onerror(new Error("Failed to load image from \"".concat(src, "\" (error ").concat(code, ")")));
        } else {
          if (this.prop("data", data)) onload(this);else onerror(new Error("Could not decode image data"));
        }
      });
    } else {
      // local file path
      data = fs.readFileSync(src);
    }

    this.prop("src", src);

    if (data) {
      if (this.prop("data", data)) onload(this);else onerror(new Error("Could not decode image data"));
    }
  }

  decode() {
    return this._fetch ? new Promise((res, rej) => this._fetch.once('ok', res).once('err', rej)) : this.complete ? Promise.resolve(this) : Promise.reject(new Error("Missing Source URL"));
  }

  [REPR](depth, options) {
    let {
      width,
      height,
      complete,
      src
    } = this;
    options.maxStringLength = src.match(/^data:/) ? 128 : Infinity;
    return "Image ".concat(inspect({
      width,
      height,
      complete,
      src
    }, options));
  }

}

class ImageData {
  constructor() {
    for (var _len9 = arguments.length, args = new Array(_len9), _key9 = 0; _key9 < _len9; _key9++) {
      args[_key9] = arguments[_key9];
    }

    if (args[0] instanceof ImageData) {
      var {
        data,
        width,
        height
      } = args[0];
    } else if (args[0] instanceof Uint8ClampedArray || args[0] instanceof Buffer) {
      var [data, width, height] = args;
      height = height || data.length / width / 4;

      if (data.length / 4 != width * height) {
        throw new Error("ImageData dimensions must match buffer length");
      }
    } else {
      var [width, height] = args;
    }

    if (!Number.isInteger(width) || !Number.isInteger(height) || width < 0 || height < 0) {
      throw new Error("ImageData dimensions must be positive integers");
    }

    readOnly(this, "width", width);
    readOnly(this, "height", height);
    readOnly(this, "data", new Uint8ClampedArray(data && data.buffer || width * height * 4));
  }

  [REPR](depth, options) {
    let {
      width,
      height,
      data
    } = this;
    return "ImageData ".concat(inspect({
      width,
      height,
      data
    }, options));
  }

}

class Path2D extends RustClass {
  static op(operation, path, other) {
    return wrap(Path2D, path.ƒ("op", core(other), operation));
  }

  static interpolate(path, other, weight) {
    return wrap(Path2D, path.ƒ("interpolate", core(other), weight));
  }

  static effect(effect, path) {
    for (var _len10 = arguments.length, args = new Array(_len10 > 2 ? _len10 - 2 : 0), _key10 = 2; _key10 < _len10; _key10++) {
      args[_key10 - 2] = arguments[_key10];
    }

    return wrap(Path2D, path.ƒ(effect, ...args));
  }

  constructor(source) {
    super(Path2D);
    if (source instanceof Path2D) this.init('from_path', core(source));else if (typeof source == 'string') this.init('from_svg', source);else this.alloc();
  } // dimensions & contents


  get bounds() {
    return this.ƒ('bounds');
  }

  get edges() {
    return this.ƒ("edges");
  }

  get d() {
    return this.prop("d");
  }

  set d(svg) {
    return this.prop("d", svg);
  }

  contains(x, y) {
    return this.ƒ("contains", x, y);
  }

  points() {
    let step = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 1;
    return this.jitter(step, 0).edges.map(_ref2 => {
      let [verb, ...pts] = _ref2;
      return pts.slice(-2);
    }).filter(pt => pt.length);
  } // concatenation


  addPath(path, matrix) {
    if (!(path instanceof Path2D)) throw new Error("Expected a Path2D object");
    if (matrix) matrix = toSkMatrix(matrix);
    this.ƒ('addPath', core(path), matrix);
  } // line segments


  moveTo(x, y) {
    this.ƒ("moveTo", x, y);
  }

  lineTo(x, y) {
    this.ƒ("lineTo", x, y);
  }

  closePath() {
    this.ƒ("closePath");
  }

  arcTo(x1, y1, x2, y2, radius) {
    this.ƒ("arcTo", ...arguments);
  }

  bezierCurveTo(cp1x, cp1y, cp2x, cp2y, x, y) {
    this.ƒ("bezierCurveTo", ...arguments);
  }

  quadraticCurveTo(cpx, cpy, x, y) {
    this.ƒ("quadraticCurveTo", ...arguments);
  }

  conicCurveTo(cpx, cpy, x, y, weight) {
    this.ƒ("conicCurveTo", ...arguments);
  } // shape primitives


  ellipse(x, y, radiusX, radiusY, rotation, startAngle, endAngle, isCCW) {
    this.ƒ("ellipse", ...arguments);
  }

  rect(x, y, width, height) {
    this.ƒ("rect", ...arguments);
  }

  arc(x, y, radius, startAngle, endAngle) {
    this.ƒ("arc", ...arguments);
  } // tween similar paths


  interpolate(path, weight) {
    return Path2D.interpolate(this, path, weight);
  } // boolean operations


  complement(path) {
    return Path2D.op("complement", this, path);
  }

  difference(path) {
    return Path2D.op("difference", this, path);
  }

  intersect(path) {
    return Path2D.op("intersect", this, path);
  }

  union(path) {
    return Path2D.op("union", this, path);
  }

  xor(path) {
    return Path2D.op("xor", this, path);
  } // path effects


  jitter(len, amt, seed) {
    return Path2D.effect("jitter", this, ...arguments);
  }

  simplify(rule) {
    return Path2D.effect("simplify", this, rule);
  }

  unwind() {
    return Path2D.effect("unwind", this);
  }

  round(radius) {
    return Path2D.effect("round", this, radius);
  }

  offset(dx, dy) {
    return Path2D.effect("offset", this, dx, dy);
  }

  transform(matrix) {
    let terms = arguments.length > 1 ? [...arguments] : matrix;
    return Path2D.effect("transform", this, toSkMatrix(terms));
  }

  trim() {
    for (var _len11 = arguments.length, rng = new Array(_len11), _key11 = 0; _key11 < _len11; _key11++) {
      rng[_key11] = arguments[_key11];
    }

    if (typeof rng[1] != 'number') {
      if (rng[0] > 0) rng.unshift(0);else if (rng[0] < 0) rng.splice(1, 0, 1);
    }

    if (rng[0] < 0) rng[0] = Math.max(-1, rng[0]) + 1;
    if (rng[1] < 0) rng[1] = Math.max(-1, rng[1]) + 1;
    return Path2D.effect("trim", this, ...rng);
  }

  [REPR](depth, options) {
    let {
      d,
      bounds,
      edges
    } = this;
    return "Path2D ".concat(inspect({
      d,
      bounds,
      edges
    }, options));
  }

}

class TextMetrics {
  constructor(_ref3, lines) {
    let [width, left, right, ascent, descent, fontAscent, fontDescent, emAscent, emDescent, hanging, alphabetic, ideographic] = _ref3;
    readOnly(this, "width", width);
    readOnly(this, "actualBoundingBoxLeft", left);
    readOnly(this, "actualBoundingBoxRight", right);
    readOnly(this, "actualBoundingBoxAscent", ascent);
    readOnly(this, "actualBoundingBoxDescent", descent);
    readOnly(this, "fontBoundingBoxAscent", fontAscent);
    readOnly(this, "fontBoundingBoxDescent", fontDescent);
    readOnly(this, "emHeightAscent", emAscent);
    readOnly(this, "emHeightDescent", emDescent);
    readOnly(this, "hangingBaseline", hanging);
    readOnly(this, "alphabeticBaseline", alphabetic);
    readOnly(this, "ideographicBaseline", ideographic);
    readOnly(this, "lines", lines.map(_ref4 => {
      let [x, y, width, height, baseline, startIndex, endIndex] = _ref4;
      return {
        x,
        y,
        width,
        height,
        baseline,
        startIndex,
        endIndex
      };
    }));
  }

}

const loadImage = src => new Promise((onload, onerror) => Object.assign(new Image(), {
  onload,
  onerror,
  src
}));

module.exports = _objectSpread(_objectSpread({
  Canvas,
  CanvasGradient,
  CanvasPattern,
  CanvasRenderingContext2D,
  CanvasTexture,
  TextMetrics,
  Image,
  ImageData,
  Path2D,
  loadImage
}, geometry), {}, {
  FontLibrary: new FontLibrary()
});