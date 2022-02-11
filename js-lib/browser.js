"use strict";

require("core-js/modules/es6.symbol.js");

require("core-js/modules/web.dom.iterable.js");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) { symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); } keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { _defineProperty(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

function _defineProperty(obj, key, value) { if (key in obj) { Object.defineProperty(obj, key, { value: value, enumerable: true, configurable: true, writable: true }); } else { obj[key] = value; } return obj; }

const {
  asBuffer,
  asDownload,
  asZipDownload,
  atScale,
  options
} = require('./io'); //
// Browser equivalents of the skia-canvas convenience initializers and polyfills for
// the Canvas object’s newPage & export methods
//


const _toURL_ = Symbol.for("toDataURL");

const loadImage = src => new Promise((onload, onerror) => Object.assign(new Image(), {
  crossOrigin: 'Anonymous',
  onload,
  onerror,
  src
}));

class Canvas {
  constructor(width, height) {
    let elt = document.createElement('canvas'),
        _pages = [];
    Object.defineProperty(elt, "async", {
      value: true,
      writable: false,
      enumerable: true
    });

    for (var [prop, get] of Object.entries({
      png: () => asBuffer(elt, 'image/png'),
      jpg: () => asBuffer(elt, 'image/jpeg'),
      pages: () => _pages.concat(elt).map(c => c.getContext("2d"))
    })) Object.defineProperty(elt, prop, {
      get
    });

    return Object.assign(elt, {
      width,
      height,

      newPage() {
        var {
          width,
          height
        } = elt,
            page = Object.assign(document.createElement('canvas'), {
          width,
          height
        });
        page.getContext("2d").drawImage(elt, 0, 0);

        _pages.push(page);

        for (var _len = arguments.length, size = new Array(_len), _key = 0; _key < _len; _key++) {
          size[_key] = arguments[_key];
        }

        var [width, height] = size.length ? size : [width, height];
        return Object.assign(elt, {
          width,
          height
        }).getContext("2d");
      },

      saveAs(filename, args) {
        args = typeof args == 'number' ? {
          quality: args
        } : args;
        let opts = options(this.pages, _objectSpread({
          filename
        }, args)),
            {
          pattern,
          padding,
          mime,
          quality,
          matte,
          density,
          archive
        } = opts,
            pages = atScale(opts.pages, density);
        return padding == undefined ? asDownload(pages[0], mime, quality, matte, filename) : asZipDownload(pages, mime, quality, matte, archive, pattern, padding);
      },

      toBuffer() {
        let extension = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : "png";
        let args = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
        args = typeof args == 'number' ? {
          quality: args
        } : args;
        let opts = options(this.pages, _objectSpread({
          extension
        }, args)),
            {
          mime,
          quality,
          matte,
          pages,
          density
        } = opts,
            canvas = atScale(pages, density, matte)[0];
        return asBuffer(canvas, mime, quality, matte);
      },

      [_toURL_]: elt.toDataURL.bind(elt),

      toDataURL() {
        let extension = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : "png";
        let args = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
        args = typeof args == 'number' ? {
          quality: args
        } : args;
        let opts = options(this.pages, _objectSpread({
          extension
        }, args)),
            {
          mime,
          quality,
          matte,
          pages,
          density
        } = opts,
            canvas = atScale(pages, density, matte)[0],
            url = canvas[canvas === elt ? _toURL_ : 'toDataURL'](mime, quality);
        return Promise.resolve(url);
      }

    });
  }

}

const {
  CanvasRenderingContext2D,
  CanvasGradient,
  CanvasPattern,
  Image,
  ImageData,
  Path2D,
  DOMMatrix,
  DOMRect,
  DOMPoint
} = window;
module.exports = {
  Canvas,
  loadImage,
  CanvasRenderingContext2D,
  CanvasGradient,
  CanvasPattern,
  Image,
  ImageData,
  Path2D,
  DOMMatrix,
  DOMRect,
  DOMPoint
};