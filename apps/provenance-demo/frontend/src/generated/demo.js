/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-mixed-operators, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars, default-case, jsdoc/require-param*/
import $protobuf from "protobufjs/minimal.js";

// Common aliases
const $Reader = $protobuf.Reader, $Writer = $protobuf.Writer, $util = $protobuf.util;
const $Object = $util.global.Object, $undefined = $util.global.undefined, $Error = $util.global.Error, $RangeError = $util.global.RangeError, $TypeError = $util.global.TypeError, $Number = $util.global.Number, $parseInt = $util.global.parseInt, $String = $util.global.String, $BigInt = $util.global.BigInt, $Array = $util.global.Array;

// Exported root namespace
const $root = $protobuf.roots["default"] || ($protobuf.roots["default"] = {});

export const doramcap = $root.doramcap = (() => {

    /**
     * Namespace doramcap.
     * @exports doramcap
     * @namespace
     */
    const doramcap = {};

    doramcap.provenance_demo = (function() {

        /**
         * Namespace provenance_demo.
         * @memberof doramcap
         * @namespace
         */
        const provenance_demo = {};

        provenance_demo.v1 = (function() {

            /**
             * Namespace v1.
             * @memberof doramcap.provenance_demo
             * @namespace
             */
            const v1 = {};

            v1.Payload = (function() {

                /**
                 * Properties of a Payload.
                 * @typedef {Object} doramcap.provenance_demo.v1.Payload.$Properties
                 * @property {doramcap.provenance.v1.GenerationRef.$Properties|null} [generation] Payload generation
                 * @property {number|Long|null} [producedNs] Payload producedNs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Payload.
                 * @memberof doramcap.provenance_demo.v1
                 * @interface IPayload
                 * @augments doramcap.provenance_demo.v1.Payload.$Properties
                 * @deprecated Use doramcap.provenance_demo.v1.Payload.$Properties instead.
                 */

                /**
                 * Shape of a Payload.
                 * @typedef {doramcap.provenance_demo.v1.Payload.$Properties} doramcap.provenance_demo.v1.Payload.$Shape
                 */

                /**
                 * Constructs a new Payload.
                 * @memberof doramcap.provenance_demo.v1
                 * @classdesc Represents a Payload.
                 * @constructor
                 * @param {doramcap.provenance_demo.v1.Payload.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Payload = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Payload generation.
                 * @member {doramcap.provenance.v1.GenerationRef.$Properties|null|undefined} generation
                 * @memberof doramcap.provenance_demo.v1.Payload
                 * @instance
                 */
                Payload.prototype.generation = null;

                /**
                 * Payload producedNs.
                 * @member {number|Long} producedNs
                 * @memberof doramcap.provenance_demo.v1.Payload
                 * @instance
                 */
                Payload.prototype.producedNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified Payload message. Does not implicitly {@link doramcap.provenance_demo.v1.Payload.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance_demo.v1.Payload
                 * @static
                 * @param {doramcap.provenance_demo.v1.Payload.$Properties} message Payload message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Payload.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.generation != null && $Object.hasOwnProperty.call(message, "generation"))
                        $root.doramcap.provenance.v1.GenerationRef.encode(message.generation, writer.uint32(/* id 1, wireType 2 =*/10).fork(), _depth + 1).ldelim();
                    if (message.producedNs != null && $Object.hasOwnProperty.call(message, "producedNs") && (typeof message.producedNs === "object" ? message.producedNs.low || message.producedNs.high : message.producedNs !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.producedNs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Payload message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance_demo.v1.Payload
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance_demo.v1.Payload & doramcap.provenance_demo.v1.Payload.$Shape} Payload
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Payload.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance_demo.v1.Payload();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 2)
                                    break;
                                message.generation = $root.doramcap.provenance.v1.GenerationRef.decode(reader, reader.size(), $undefined, _depth + 1, message.generation);
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.producedNs = value;
                                else
                                    delete message.producedNs;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a Payload message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance_demo.v1.Payload
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance_demo.v1.Payload} Payload
                 */
                Payload.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance_demo.v1.Payload)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance_demo.v1.Payload: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance_demo.v1.Payload();
                    if (object.generation != null) {
                        if (!$util.isObject(object.generation))
                            throw $TypeError(".doramcap.provenance_demo.v1.Payload.generation: object expected");
                        message.generation = $root.doramcap.provenance.v1.GenerationRef.fromObject(object.generation, _depth + 1);
                    }
                    if (object.producedNs != null)
                        if (typeof object.producedNs === "object" ? object.producedNs.low || object.producedNs.high : $Number(object.producedNs) !== 0)
                            if ($util.Long)
                                message.producedNs = $util.Long.fromValue(object.producedNs, true);
                            else if (typeof object.producedNs === "string")
                                message.producedNs = $parseInt(object.producedNs, 10);
                            else if (typeof object.producedNs === "number")
                                message.producedNs = object.producedNs;
                            else if (typeof object.producedNs === "object")
                                message.producedNs = new $util.LongBits(object.producedNs.low >>> 0, object.producedNs.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a Payload message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance_demo.v1.Payload
                 * @static
                 * @param {doramcap.provenance_demo.v1.Payload} message Payload
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Payload.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.generation = null;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.producedNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.producedNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.generation != null && $Object.hasOwnProperty.call(message, "generation"))
                        object.generation = $root.doramcap.provenance.v1.GenerationRef.toObject(message.generation, options, _depth + 1);
                    if (message.producedNs != null && $Object.hasOwnProperty.call(message, "producedNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.producedNs = typeof message.producedNs === "number" ? $BigInt(message.producedNs) : $util.Long.fromBits(message.producedNs.low >>> 0, message.producedNs.high >>> 0, true).toBigInt();
                        else if (typeof message.producedNs === "number")
                            object.producedNs = options.longs === $String ? $String(message.producedNs) : message.producedNs;
                        else
                            object.producedNs = options.longs === $String ? $util.Long.prototype.toString.call(message.producedNs) : options.longs === $Number ? new $util.LongBits(message.producedNs.low >>> 0, message.producedNs.high >>> 0).toNumber(true) : message.producedNs;
                    return object;
                };

                /**
                 * Converts this Payload to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance_demo.v1.Payload
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Payload.prototype.toJSON = function() {
                    return Payload.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Payload
                 * @function getTypeUrl
                 * @memberof doramcap.provenance_demo.v1.Payload
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Payload.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance_demo.v1.Payload";
                };

                return Payload;
            })();

            v1.Clock = (function() {

                /**
                 * Properties of a Clock.
                 * @typedef {Object} doramcap.provenance_demo.v1.Clock.$Properties
                 * @property {number|Long|null} [session] Clock session
                 * @property {number|Long|null} [unixOriginNs] Clock unixOriginNs
                 * @property {string|null} [domain] Clock domain
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Clock.
                 * @memberof doramcap.provenance_demo.v1
                 * @interface IClock
                 * @augments doramcap.provenance_demo.v1.Clock.$Properties
                 * @deprecated Use doramcap.provenance_demo.v1.Clock.$Properties instead.
                 */

                /**
                 * Shape of a Clock.
                 * @typedef {doramcap.provenance_demo.v1.Clock.$Properties} doramcap.provenance_demo.v1.Clock.$Shape
                 */

                /**
                 * Constructs a new Clock.
                 * @memberof doramcap.provenance_demo.v1
                 * @classdesc Represents a Clock.
                 * @constructor
                 * @param {doramcap.provenance_demo.v1.Clock.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Clock = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Clock session.
                 * @member {number|Long} session
                 * @memberof doramcap.provenance_demo.v1.Clock
                 * @instance
                 */
                Clock.prototype.session = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Clock unixOriginNs.
                 * @member {number|Long} unixOriginNs
                 * @memberof doramcap.provenance_demo.v1.Clock
                 * @instance
                 */
                Clock.prototype.unixOriginNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Clock domain.
                 * @member {string} domain
                 * @memberof doramcap.provenance_demo.v1.Clock
                 * @instance
                 */
                Clock.prototype.domain = "";

                /**
                 * Encodes the specified Clock message. Does not implicitly {@link doramcap.provenance_demo.v1.Clock.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance_demo.v1.Clock
                 * @static
                 * @param {doramcap.provenance_demo.v1.Clock.$Properties} message Clock message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Clock.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.session != null && $Object.hasOwnProperty.call(message, "session") && (typeof message.session === "object" ? message.session.low || message.session.high : message.session !== 0))
                        writer.uint32(/* id 1, wireType 1 =*/9).fixed64(message.session);
                    if (message.unixOriginNs != null && $Object.hasOwnProperty.call(message, "unixOriginNs") && (typeof message.unixOriginNs === "object" ? message.unixOriginNs.low || message.unixOriginNs.high : message.unixOriginNs !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.unixOriginNs);
                    if (message.domain != null && $Object.hasOwnProperty.call(message, "domain") && message.domain !== "")
                        writer.uint32(/* id 3, wireType 2 =*/26).string(message.domain);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Clock message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance_demo.v1.Clock
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance_demo.v1.Clock & doramcap.provenance_demo.v1.Clock.$Shape} Clock
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Clock.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance_demo.v1.Clock();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 1)
                                    break;
                                if (typeof (value = reader.fixed64()) === "object" ? value.low || value.high : value !== 0)
                                    message.session = value;
                                else
                                    delete message.session;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.unixOriginNs = value;
                                else
                                    delete message.unixOriginNs;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.domain = value;
                                else
                                    delete message.domain;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a Clock message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance_demo.v1.Clock
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance_demo.v1.Clock} Clock
                 */
                Clock.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance_demo.v1.Clock)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance_demo.v1.Clock: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance_demo.v1.Clock();
                    if (object.session != null)
                        if (typeof object.session === "object" ? object.session.low || object.session.high : $Number(object.session) !== 0)
                            if ($util.Long)
                                message.session = $util.Long.fromValue(object.session, true);
                            else if (typeof object.session === "string")
                                message.session = $parseInt(object.session, 10);
                            else if (typeof object.session === "number")
                                message.session = object.session;
                            else if (typeof object.session === "object")
                                message.session = new $util.LongBits(object.session.low >>> 0, object.session.high >>> 0).toNumber(true);
                    if (object.unixOriginNs != null)
                        if (typeof object.unixOriginNs === "object" ? object.unixOriginNs.low || object.unixOriginNs.high : $Number(object.unixOriginNs) !== 0)
                            if ($util.Long)
                                message.unixOriginNs = $util.Long.fromValue(object.unixOriginNs, true);
                            else if (typeof object.unixOriginNs === "string")
                                message.unixOriginNs = $parseInt(object.unixOriginNs, 10);
                            else if (typeof object.unixOriginNs === "number")
                                message.unixOriginNs = object.unixOriginNs;
                            else if (typeof object.unixOriginNs === "object")
                                message.unixOriginNs = new $util.LongBits(object.unixOriginNs.low >>> 0, object.unixOriginNs.high >>> 0).toNumber(true);
                    if (object.domain != null)
                        if (typeof object.domain !== "string" || object.domain.length)
                            message.domain = $String(object.domain);
                    return message;
                };

                /**
                 * Creates a plain object from a Clock message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance_demo.v1.Clock
                 * @static
                 * @param {doramcap.provenance_demo.v1.Clock} message Clock
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Clock.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.session = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.session = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.unixOriginNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.unixOriginNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.domain = "";
                    }
                    if (message.session != null && $Object.hasOwnProperty.call(message, "session"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.session = typeof message.session === "number" ? $BigInt(message.session) : $util.Long.fromBits(message.session.low >>> 0, message.session.high >>> 0, true).toBigInt();
                        else if (typeof message.session === "number")
                            object.session = options.longs === $String ? $String(message.session) : message.session;
                        else
                            object.session = options.longs === $String ? $util.Long.prototype.toString.call(message.session) : options.longs === $Number ? new $util.LongBits(message.session.low >>> 0, message.session.high >>> 0).toNumber(true) : message.session;
                    if (message.unixOriginNs != null && $Object.hasOwnProperty.call(message, "unixOriginNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.unixOriginNs = typeof message.unixOriginNs === "number" ? $BigInt(message.unixOriginNs) : $util.Long.fromBits(message.unixOriginNs.low >>> 0, message.unixOriginNs.high >>> 0, true).toBigInt();
                        else if (typeof message.unixOriginNs === "number")
                            object.unixOriginNs = options.longs === $String ? $String(message.unixOriginNs) : message.unixOriginNs;
                        else
                            object.unixOriginNs = options.longs === $String ? $util.Long.prototype.toString.call(message.unixOriginNs) : options.longs === $Number ? new $util.LongBits(message.unixOriginNs.low >>> 0, message.unixOriginNs.high >>> 0).toNumber(true) : message.unixOriginNs;
                    if (message.domain != null && $Object.hasOwnProperty.call(message, "domain"))
                        object.domain = message.domain;
                    return object;
                };

                /**
                 * Converts this Clock to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance_demo.v1.Clock
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Clock.prototype.toJSON = function() {
                    return Clock.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Clock
                 * @function getTypeUrl
                 * @memberof doramcap.provenance_demo.v1.Clock
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Clock.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance_demo.v1.Clock";
                };

                return Clock;
            })();

            v1.NodeStatus = (function() {

                /**
                 * Properties of a NodeStatus.
                 * @typedef {Object} doramcap.provenance_demo.v1.NodeStatus.$Properties
                 * @property {string|null} [node] NodeStatus node
                 * @property {number|null} [pid] NodeStatus pid
                 * @property {string|null} [epoch] NodeStatus epoch
                 * @property {number|Long|null} [activityQueueDrops] NodeStatus activityQueueDrops
                 * @property {number|Long|null} [simulatedReportDrops] NodeStatus simulatedReportDrops
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a NodeStatus.
                 * @memberof doramcap.provenance_demo.v1
                 * @interface INodeStatus
                 * @augments doramcap.provenance_demo.v1.NodeStatus.$Properties
                 * @deprecated Use doramcap.provenance_demo.v1.NodeStatus.$Properties instead.
                 */

                /**
                 * Shape of a NodeStatus.
                 * @typedef {doramcap.provenance_demo.v1.NodeStatus.$Properties} doramcap.provenance_demo.v1.NodeStatus.$Shape
                 */

                /**
                 * Constructs a new NodeStatus.
                 * @memberof doramcap.provenance_demo.v1
                 * @classdesc Represents a NodeStatus.
                 * @constructor
                 * @param {doramcap.provenance_demo.v1.NodeStatus.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const NodeStatus = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * NodeStatus node.
                 * @member {string} node
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @instance
                 */
                NodeStatus.prototype.node = "";

                /**
                 * NodeStatus pid.
                 * @member {number} pid
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @instance
                 */
                NodeStatus.prototype.pid = 0;

                /**
                 * NodeStatus epoch.
                 * @member {string} epoch
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @instance
                 */
                NodeStatus.prototype.epoch = "";

                /**
                 * NodeStatus activityQueueDrops.
                 * @member {number|Long} activityQueueDrops
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @instance
                 */
                NodeStatus.prototype.activityQueueDrops = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * NodeStatus simulatedReportDrops.
                 * @member {number|Long} simulatedReportDrops
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @instance
                 */
                NodeStatus.prototype.simulatedReportDrops = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified NodeStatus message. Does not implicitly {@link doramcap.provenance_demo.v1.NodeStatus.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @static
                 * @param {doramcap.provenance_demo.v1.NodeStatus.$Properties} message NodeStatus message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                NodeStatus.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.node != null && $Object.hasOwnProperty.call(message, "node") && message.node !== "")
                        writer.uint32(/* id 1, wireType 2 =*/10).string(message.node);
                    if (message.pid != null && $Object.hasOwnProperty.call(message, "pid") && message.pid !== 0)
                        writer.uint32(/* id 2, wireType 0 =*/16).uint32(message.pid);
                    if (message.epoch != null && $Object.hasOwnProperty.call(message, "epoch") && message.epoch !== "")
                        writer.uint32(/* id 3, wireType 2 =*/26).string(message.epoch);
                    if (message.activityQueueDrops != null && $Object.hasOwnProperty.call(message, "activityQueueDrops") && (typeof message.activityQueueDrops === "object" ? message.activityQueueDrops.low || message.activityQueueDrops.high : message.activityQueueDrops !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.activityQueueDrops);
                    if (message.simulatedReportDrops != null && $Object.hasOwnProperty.call(message, "simulatedReportDrops") && (typeof message.simulatedReportDrops === "object" ? message.simulatedReportDrops.low || message.simulatedReportDrops.high : message.simulatedReportDrops !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.simulatedReportDrops);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a NodeStatus message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance_demo.v1.NodeStatus & doramcap.provenance_demo.v1.NodeStatus.$Shape} NodeStatus
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                NodeStatus.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance_demo.v1.NodeStatus();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.node = value;
                                else
                                    delete message.node;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.pid = value;
                                else
                                    delete message.pid;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.epoch = value;
                                else
                                    delete message.epoch;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.activityQueueDrops = value;
                                else
                                    delete message.activityQueueDrops;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.simulatedReportDrops = value;
                                else
                                    delete message.simulatedReportDrops;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a NodeStatus message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance_demo.v1.NodeStatus} NodeStatus
                 */
                NodeStatus.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance_demo.v1.NodeStatus)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance_demo.v1.NodeStatus: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance_demo.v1.NodeStatus();
                    if (object.node != null)
                        if (typeof object.node !== "string" || object.node.length)
                            message.node = $String(object.node);
                    if (object.pid != null)
                        if ($Number(object.pid) !== 0)
                            message.pid = object.pid >>> 0;
                    if (object.epoch != null)
                        if (typeof object.epoch !== "string" || object.epoch.length)
                            message.epoch = $String(object.epoch);
                    if (object.activityQueueDrops != null)
                        if (typeof object.activityQueueDrops === "object" ? object.activityQueueDrops.low || object.activityQueueDrops.high : $Number(object.activityQueueDrops) !== 0)
                            if ($util.Long)
                                message.activityQueueDrops = $util.Long.fromValue(object.activityQueueDrops, true);
                            else if (typeof object.activityQueueDrops === "string")
                                message.activityQueueDrops = $parseInt(object.activityQueueDrops, 10);
                            else if (typeof object.activityQueueDrops === "number")
                                message.activityQueueDrops = object.activityQueueDrops;
                            else if (typeof object.activityQueueDrops === "object")
                                message.activityQueueDrops = new $util.LongBits(object.activityQueueDrops.low >>> 0, object.activityQueueDrops.high >>> 0).toNumber(true);
                    if (object.simulatedReportDrops != null)
                        if (typeof object.simulatedReportDrops === "object" ? object.simulatedReportDrops.low || object.simulatedReportDrops.high : $Number(object.simulatedReportDrops) !== 0)
                            if ($util.Long)
                                message.simulatedReportDrops = $util.Long.fromValue(object.simulatedReportDrops, true);
                            else if (typeof object.simulatedReportDrops === "string")
                                message.simulatedReportDrops = $parseInt(object.simulatedReportDrops, 10);
                            else if (typeof object.simulatedReportDrops === "number")
                                message.simulatedReportDrops = object.simulatedReportDrops;
                            else if (typeof object.simulatedReportDrops === "object")
                                message.simulatedReportDrops = new $util.LongBits(object.simulatedReportDrops.low >>> 0, object.simulatedReportDrops.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a NodeStatus message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @static
                 * @param {doramcap.provenance_demo.v1.NodeStatus} message NodeStatus
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                NodeStatus.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.node = "";
                        object.pid = 0;
                        object.epoch = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.activityQueueDrops = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.activityQueueDrops = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.simulatedReportDrops = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.simulatedReportDrops = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.node != null && $Object.hasOwnProperty.call(message, "node"))
                        object.node = message.node;
                    if (message.pid != null && $Object.hasOwnProperty.call(message, "pid"))
                        object.pid = message.pid;
                    if (message.epoch != null && $Object.hasOwnProperty.call(message, "epoch"))
                        object.epoch = message.epoch;
                    if (message.activityQueueDrops != null && $Object.hasOwnProperty.call(message, "activityQueueDrops"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.activityQueueDrops = typeof message.activityQueueDrops === "number" ? $BigInt(message.activityQueueDrops) : $util.Long.fromBits(message.activityQueueDrops.low >>> 0, message.activityQueueDrops.high >>> 0, true).toBigInt();
                        else if (typeof message.activityQueueDrops === "number")
                            object.activityQueueDrops = options.longs === $String ? $String(message.activityQueueDrops) : message.activityQueueDrops;
                        else
                            object.activityQueueDrops = options.longs === $String ? $util.Long.prototype.toString.call(message.activityQueueDrops) : options.longs === $Number ? new $util.LongBits(message.activityQueueDrops.low >>> 0, message.activityQueueDrops.high >>> 0).toNumber(true) : message.activityQueueDrops;
                    if (message.simulatedReportDrops != null && $Object.hasOwnProperty.call(message, "simulatedReportDrops"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.simulatedReportDrops = typeof message.simulatedReportDrops === "number" ? $BigInt(message.simulatedReportDrops) : $util.Long.fromBits(message.simulatedReportDrops.low >>> 0, message.simulatedReportDrops.high >>> 0, true).toBigInt();
                        else if (typeof message.simulatedReportDrops === "number")
                            object.simulatedReportDrops = options.longs === $String ? $String(message.simulatedReportDrops) : message.simulatedReportDrops;
                        else
                            object.simulatedReportDrops = options.longs === $String ? $util.Long.prototype.toString.call(message.simulatedReportDrops) : options.longs === $Number ? new $util.LongBits(message.simulatedReportDrops.low >>> 0, message.simulatedReportDrops.high >>> 0).toNumber(true) : message.simulatedReportDrops;
                    return object;
                };

                /**
                 * Converts this NodeStatus to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                NodeStatus.prototype.toJSON = function() {
                    return NodeStatus.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for NodeStatus
                 * @function getTypeUrl
                 * @memberof doramcap.provenance_demo.v1.NodeStatus
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                NodeStatus.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance_demo.v1.NodeStatus";
                };

                return NodeStatus;
            })();

            v1.Gap = (function() {

                /**
                 * Properties of a Gap.
                 * @typedef {Object} doramcap.provenance_demo.v1.Gap.$Properties
                 * @property {number|Long|null} [envelopes] Gap envelopes
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Gap.
                 * @memberof doramcap.provenance_demo.v1
                 * @interface IGap
                 * @augments doramcap.provenance_demo.v1.Gap.$Properties
                 * @deprecated Use doramcap.provenance_demo.v1.Gap.$Properties instead.
                 */

                /**
                 * Shape of a Gap.
                 * @typedef {doramcap.provenance_demo.v1.Gap.$Properties} doramcap.provenance_demo.v1.Gap.$Shape
                 */

                /**
                 * Constructs a new Gap.
                 * @memberof doramcap.provenance_demo.v1
                 * @classdesc Represents a Gap.
                 * @constructor
                 * @param {doramcap.provenance_demo.v1.Gap.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Gap = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Gap envelopes.
                 * @member {number|Long} envelopes
                 * @memberof doramcap.provenance_demo.v1.Gap
                 * @instance
                 */
                Gap.prototype.envelopes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified Gap message. Does not implicitly {@link doramcap.provenance_demo.v1.Gap.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance_demo.v1.Gap
                 * @static
                 * @param {doramcap.provenance_demo.v1.Gap.$Properties} message Gap message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Gap.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.envelopes != null && $Object.hasOwnProperty.call(message, "envelopes") && (typeof message.envelopes === "object" ? message.envelopes.low || message.envelopes.high : message.envelopes !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.envelopes);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Gap message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance_demo.v1.Gap
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance_demo.v1.Gap & doramcap.provenance_demo.v1.Gap.$Shape} Gap
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Gap.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance_demo.v1.Gap();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.envelopes = value;
                                else
                                    delete message.envelopes;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a Gap message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance_demo.v1.Gap
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance_demo.v1.Gap} Gap
                 */
                Gap.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance_demo.v1.Gap)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance_demo.v1.Gap: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance_demo.v1.Gap();
                    if (object.envelopes != null)
                        if (typeof object.envelopes === "object" ? object.envelopes.low || object.envelopes.high : $Number(object.envelopes) !== 0)
                            if ($util.Long)
                                message.envelopes = $util.Long.fromValue(object.envelopes, true);
                            else if (typeof object.envelopes === "string")
                                message.envelopes = $parseInt(object.envelopes, 10);
                            else if (typeof object.envelopes === "number")
                                message.envelopes = object.envelopes;
                            else if (typeof object.envelopes === "object")
                                message.envelopes = new $util.LongBits(object.envelopes.low >>> 0, object.envelopes.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a Gap message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance_demo.v1.Gap
                 * @static
                 * @param {doramcap.provenance_demo.v1.Gap} message Gap
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Gap.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults)
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.envelopes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.envelopes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    if (message.envelopes != null && $Object.hasOwnProperty.call(message, "envelopes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.envelopes = typeof message.envelopes === "number" ? $BigInt(message.envelopes) : $util.Long.fromBits(message.envelopes.low >>> 0, message.envelopes.high >>> 0, true).toBigInt();
                        else if (typeof message.envelopes === "number")
                            object.envelopes = options.longs === $String ? $String(message.envelopes) : message.envelopes;
                        else
                            object.envelopes = options.longs === $String ? $util.Long.prototype.toString.call(message.envelopes) : options.longs === $Number ? new $util.LongBits(message.envelopes.low >>> 0, message.envelopes.high >>> 0).toNumber(true) : message.envelopes;
                    return object;
                };

                /**
                 * Converts this Gap to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance_demo.v1.Gap
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Gap.prototype.toJSON = function() {
                    return Gap.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Gap
                 * @function getTypeUrl
                 * @memberof doramcap.provenance_demo.v1.Gap
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Gap.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance_demo.v1.Gap";
                };

                return Gap;
            })();

            v1.Envelope = (function() {

                /**
                 * Properties of an Envelope.
                 * @typedef {Object} doramcap.provenance_demo.v1.Envelope.$Properties
                 * @property {number|null} [version] Envelope version
                 * @property {string|null} [streamId] Envelope streamId
                 * @property {number|Long|null} [sequence] Envelope sequence
                 * @property {doramcap.provenance.v1.StepUpdate.$Properties|null} [step] Envelope step
                 * @property {doramcap.provenance.v1.MeasurementResult.$Properties|null} [result] Envelope result
                 * @property {doramcap.activity.v1.Batch.$Properties|null} [activity] Envelope activity
                 * @property {doramcap.provenance_demo.v1.Clock.$Properties|null} [clock] Envelope clock
                 * @property {doramcap.provenance_demo.v1.NodeStatus.$Properties|null} [status] Envelope status
                 * @property {doramcap.provenance_demo.v1.Gap.$Properties|null} [gap] Envelope gap
                 * @property {"step"|"result"|"activity"|"clock"|"status"|"gap"} [body] Envelope body
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of an Envelope.
                 * @memberof doramcap.provenance_demo.v1
                 * @interface IEnvelope
                 * @augments doramcap.provenance_demo.v1.Envelope.$Properties
                 * @deprecated Use doramcap.provenance_demo.v1.Envelope.$Properties instead.
                 */

                /**
                 * Narrowed shape of an Envelope.
                 * @typedef {{
                 *   version?: number|null;
                 *   streamId?: string|null;
                 *   sequence?: number|Long|null;
                 *   step?: doramcap.provenance.v1.StepUpdate.$Shape|null;
                 *   result?: doramcap.provenance.v1.MeasurementResult.$Shape|null;
                 *   activity?: doramcap.activity.v1.Batch.$Shape|null;
                 *   clock?: doramcap.provenance_demo.v1.Clock.$Shape|null;
                 *   status?: doramcap.provenance_demo.v1.NodeStatus.$Shape|null;
                 *   gap?: doramcap.provenance_demo.v1.Gap.$Shape|null;
                 *   $unknowns?: Array.<Uint8Array>;
                 * } & (
                 *   ({ body?: undefined; step?: null; result?: null; activity?: null; clock?: null; status?: null; gap?: null }|{ body?: "step"; step: doramcap.provenance.v1.StepUpdate.$Shape; result?: null; activity?: null; clock?: null; status?: null; gap?: null }|{ body?: "result"; step?: null; result: doramcap.provenance.v1.MeasurementResult.$Shape; activity?: null; clock?: null; status?: null; gap?: null }|{ body?: "activity"; step?: null; result?: null; activity: doramcap.activity.v1.Batch.$Shape; clock?: null; status?: null; gap?: null }|{ body?: "clock"; step?: null; result?: null; activity?: null; clock: doramcap.provenance_demo.v1.Clock.$Shape; status?: null; gap?: null }|{ body?: "status"; step?: null; result?: null; activity?: null; clock?: null; status: doramcap.provenance_demo.v1.NodeStatus.$Shape; gap?: null }|{ body?: "gap"; step?: null; result?: null; activity?: null; clock?: null; status?: null; gap: doramcap.provenance_demo.v1.Gap.$Shape })
                 * )} doramcap.provenance_demo.v1.Envelope.$Shape
                 */

                /**
                 * Constructs a new Envelope.
                 * @memberof doramcap.provenance_demo.v1
                 * @classdesc Represents an Envelope.
                 * @constructor
                 * @param {doramcap.provenance_demo.v1.Envelope.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Envelope = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Envelope version.
                 * @member {number} version
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 */
                Envelope.prototype.version = 0;

                /**
                 * Envelope streamId.
                 * @member {string} streamId
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 */
                Envelope.prototype.streamId = "";

                /**
                 * Envelope sequence.
                 * @member {number|Long} sequence
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 */
                Envelope.prototype.sequence = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Envelope step.
                 * @member {doramcap.provenance.v1.StepUpdate.$Properties|null|undefined} step
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 */
                Envelope.prototype.step = null;

                /**
                 * Envelope result.
                 * @member {doramcap.provenance.v1.MeasurementResult.$Properties|null|undefined} result
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 */
                Envelope.prototype.result = null;

                /**
                 * Envelope activity.
                 * @member {doramcap.activity.v1.Batch.$Properties|null|undefined} activity
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 */
                Envelope.prototype.activity = null;

                /**
                 * Envelope clock.
                 * @member {doramcap.provenance_demo.v1.Clock.$Properties|null|undefined} clock
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 */
                Envelope.prototype.clock = null;

                /**
                 * Envelope status.
                 * @member {doramcap.provenance_demo.v1.NodeStatus.$Properties|null|undefined} status
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 */
                Envelope.prototype.status = null;

                /**
                 * Envelope gap.
                 * @member {doramcap.provenance_demo.v1.Gap.$Properties|null|undefined} gap
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 */
                Envelope.prototype.gap = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                /**
                 * Envelope body.
                 * @member {"step"|"result"|"activity"|"clock"|"status"|"gap"|undefined} body
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 */
                $Object.defineProperty(Envelope.prototype, "body", {
                    get: $util.oneOfGetter($oneOfFields = ["step", "result", "activity", "clock", "status", "gap"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Envelope message. Does not implicitly {@link doramcap.provenance_demo.v1.Envelope.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @static
                 * @param {doramcap.provenance_demo.v1.Envelope.$Properties} message Envelope message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Envelope.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version") && message.version !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.version);
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence") && (typeof message.sequence === "object" ? message.sequence.low || message.sequence.high : message.sequence !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.sequence);
                    if (message.step != null && $Object.hasOwnProperty.call(message, "step"))
                        $root.doramcap.provenance.v1.StepUpdate.encode(message.step, writer.uint32(/* id 3, wireType 2 =*/26).fork(), _depth + 1).ldelim();
                    if (message.result != null && $Object.hasOwnProperty.call(message, "result"))
                        $root.doramcap.provenance.v1.MeasurementResult.encode(message.result, writer.uint32(/* id 4, wireType 2 =*/34).fork(), _depth + 1).ldelim();
                    if (message.activity != null && $Object.hasOwnProperty.call(message, "activity"))
                        $root.doramcap.activity.v1.Batch.encode(message.activity, writer.uint32(/* id 5, wireType 2 =*/42).fork(), _depth + 1).ldelim();
                    if (message.clock != null && $Object.hasOwnProperty.call(message, "clock"))
                        $root.doramcap.provenance_demo.v1.Clock.encode(message.clock, writer.uint32(/* id 6, wireType 2 =*/50).fork(), _depth + 1).ldelim();
                    if (message.status != null && $Object.hasOwnProperty.call(message, "status"))
                        $root.doramcap.provenance_demo.v1.NodeStatus.encode(message.status, writer.uint32(/* id 7, wireType 2 =*/58).fork(), _depth + 1).ldelim();
                    if (message.gap != null && $Object.hasOwnProperty.call(message, "gap"))
                        $root.doramcap.provenance_demo.v1.Gap.encode(message.gap, writer.uint32(/* id 8, wireType 2 =*/66).fork(), _depth + 1).ldelim();
                    if (message.streamId != null && $Object.hasOwnProperty.call(message, "streamId") && message.streamId !== "")
                        writer.uint32(/* id 9, wireType 2 =*/74).string(message.streamId);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes an Envelope message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance_demo.v1.Envelope & doramcap.provenance_demo.v1.Envelope.$Shape} Envelope
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Envelope.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance_demo.v1.Envelope();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.version = value;
                                else
                                    delete message.version;
                                continue;
                            }
                        case 9: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.streamId = value;
                                else
                                    delete message.streamId;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.sequence = value;
                                else
                                    delete message.sequence;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                message.step = $root.doramcap.provenance.v1.StepUpdate.decode(reader, reader.size(), $undefined, _depth + 1, message.step);
                                message.body = "step";
                                continue;
                            }
                        case 4: {
                                if (wireType !== 2)
                                    break;
                                message.result = $root.doramcap.provenance.v1.MeasurementResult.decode(reader, reader.size(), $undefined, _depth + 1, message.result);
                                message.body = "result";
                                continue;
                            }
                        case 5: {
                                if (wireType !== 2)
                                    break;
                                message.activity = $root.doramcap.activity.v1.Batch.decode(reader, reader.size(), $undefined, _depth + 1, message.activity);
                                message.body = "activity";
                                continue;
                            }
                        case 6: {
                                if (wireType !== 2)
                                    break;
                                message.clock = $root.doramcap.provenance_demo.v1.Clock.decode(reader, reader.size(), $undefined, _depth + 1, message.clock);
                                message.body = "clock";
                                continue;
                            }
                        case 7: {
                                if (wireType !== 2)
                                    break;
                                message.status = $root.doramcap.provenance_demo.v1.NodeStatus.decode(reader, reader.size(), $undefined, _depth + 1, message.status);
                                message.body = "status";
                                continue;
                            }
                        case 8: {
                                if (wireType !== 2)
                                    break;
                                message.gap = $root.doramcap.provenance_demo.v1.Gap.decode(reader, reader.size(), $undefined, _depth + 1, message.gap);
                                message.body = "gap";
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates an Envelope message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance_demo.v1.Envelope} Envelope
                 */
                Envelope.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance_demo.v1.Envelope)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance_demo.v1.Envelope: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance_demo.v1.Envelope();
                    if (object.version != null)
                        if ($Number(object.version) !== 0)
                            message.version = object.version >>> 0;
                    if (object.streamId != null)
                        if (typeof object.streamId !== "string" || object.streamId.length)
                            message.streamId = $String(object.streamId);
                    if (object.sequence != null)
                        if (typeof object.sequence === "object" ? object.sequence.low || object.sequence.high : $Number(object.sequence) !== 0)
                            if ($util.Long)
                                message.sequence = $util.Long.fromValue(object.sequence, true);
                            else if (typeof object.sequence === "string")
                                message.sequence = $parseInt(object.sequence, 10);
                            else if (typeof object.sequence === "number")
                                message.sequence = object.sequence;
                            else if (typeof object.sequence === "object")
                                message.sequence = new $util.LongBits(object.sequence.low >>> 0, object.sequence.high >>> 0).toNumber(true);
                    if (object.step != null) {
                        if (!$util.isObject(object.step))
                            throw $TypeError(".doramcap.provenance_demo.v1.Envelope.step: object expected");
                        message.step = $root.doramcap.provenance.v1.StepUpdate.fromObject(object.step, _depth + 1);
                    }
                    if (object.result != null) {
                        if (!$util.isObject(object.result))
                            throw $TypeError(".doramcap.provenance_demo.v1.Envelope.result: object expected");
                        message.result = $root.doramcap.provenance.v1.MeasurementResult.fromObject(object.result, _depth + 1);
                    }
                    if (object.activity != null) {
                        if (!$util.isObject(object.activity))
                            throw $TypeError(".doramcap.provenance_demo.v1.Envelope.activity: object expected");
                        message.activity = $root.doramcap.activity.v1.Batch.fromObject(object.activity, _depth + 1);
                    }
                    if (object.clock != null) {
                        if (!$util.isObject(object.clock))
                            throw $TypeError(".doramcap.provenance_demo.v1.Envelope.clock: object expected");
                        message.clock = $root.doramcap.provenance_demo.v1.Clock.fromObject(object.clock, _depth + 1);
                    }
                    if (object.status != null) {
                        if (!$util.isObject(object.status))
                            throw $TypeError(".doramcap.provenance_demo.v1.Envelope.status: object expected");
                        message.status = $root.doramcap.provenance_demo.v1.NodeStatus.fromObject(object.status, _depth + 1);
                    }
                    if (object.gap != null) {
                        if (!$util.isObject(object.gap))
                            throw $TypeError(".doramcap.provenance_demo.v1.Envelope.gap: object expected");
                        message.gap = $root.doramcap.provenance_demo.v1.Gap.fromObject(object.gap, _depth + 1);
                    }
                    return message;
                };

                /**
                 * Creates a plain object from an Envelope message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @static
                 * @param {doramcap.provenance_demo.v1.Envelope} message Envelope
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Envelope.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.version = 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.sequence = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.sequence = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.streamId = "";
                    }
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version"))
                        object.version = message.version;
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.sequence = typeof message.sequence === "number" ? $BigInt(message.sequence) : $util.Long.fromBits(message.sequence.low >>> 0, message.sequence.high >>> 0, true).toBigInt();
                        else if (typeof message.sequence === "number")
                            object.sequence = options.longs === $String ? $String(message.sequence) : message.sequence;
                        else
                            object.sequence = options.longs === $String ? $util.Long.prototype.toString.call(message.sequence) : options.longs === $Number ? new $util.LongBits(message.sequence.low >>> 0, message.sequence.high >>> 0).toNumber(true) : message.sequence;
                    if (message.step != null && $Object.hasOwnProperty.call(message, "step")) {
                        object.step = $root.doramcap.provenance.v1.StepUpdate.toObject(message.step, options, _depth + 1);
                        if (options.oneofs)
                            object.body = "step";
                    }
                    if (message.result != null && $Object.hasOwnProperty.call(message, "result")) {
                        object.result = $root.doramcap.provenance.v1.MeasurementResult.toObject(message.result, options, _depth + 1);
                        if (options.oneofs)
                            object.body = "result";
                    }
                    if (message.activity != null && $Object.hasOwnProperty.call(message, "activity")) {
                        object.activity = $root.doramcap.activity.v1.Batch.toObject(message.activity, options, _depth + 1);
                        if (options.oneofs)
                            object.body = "activity";
                    }
                    if (message.clock != null && $Object.hasOwnProperty.call(message, "clock")) {
                        object.clock = $root.doramcap.provenance_demo.v1.Clock.toObject(message.clock, options, _depth + 1);
                        if (options.oneofs)
                            object.body = "clock";
                    }
                    if (message.status != null && $Object.hasOwnProperty.call(message, "status")) {
                        object.status = $root.doramcap.provenance_demo.v1.NodeStatus.toObject(message.status, options, _depth + 1);
                        if (options.oneofs)
                            object.body = "status";
                    }
                    if (message.gap != null && $Object.hasOwnProperty.call(message, "gap")) {
                        object.gap = $root.doramcap.provenance_demo.v1.Gap.toObject(message.gap, options, _depth + 1);
                        if (options.oneofs)
                            object.body = "gap";
                    }
                    if (message.streamId != null && $Object.hasOwnProperty.call(message, "streamId"))
                        object.streamId = message.streamId;
                    return object;
                };

                /**
                 * Converts this Envelope to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Envelope.prototype.toJSON = function() {
                    return Envelope.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Envelope
                 * @function getTypeUrl
                 * @memberof doramcap.provenance_demo.v1.Envelope
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Envelope.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance_demo.v1.Envelope";
                };

                return Envelope;
            })();

            return v1;
        })();

        return provenance_demo;
    })();

    doramcap.provenance = (function() {

        /**
         * Namespace provenance.
         * @memberof doramcap
         * @namespace
         */
        const provenance = {};

        provenance.v1 = (function() {

            /**
             * Namespace v1.
             * @memberof doramcap.provenance
             * @namespace
             */
            const v1 = {};

            v1.Step = (function() {

                /**
                 * Properties of a Step.
                 * @typedef {Object} doramcap.provenance.v1.Step.$Properties
                 * @property {string|null} [producer] Step producer
                 * @property {string|null} [name] Step name
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Step.
                 * @memberof doramcap.provenance.v1
                 * @interface IStep
                 * @augments doramcap.provenance.v1.Step.$Properties
                 * @deprecated Use doramcap.provenance.v1.Step.$Properties instead.
                 */

                /**
                 * Shape of a Step.
                 * @typedef {doramcap.provenance.v1.Step.$Properties} doramcap.provenance.v1.Step.$Shape
                 */

                /**
                 * Constructs a new Step.
                 * @memberof doramcap.provenance.v1
                 * @classdesc Represents a Step.
                 * @constructor
                 * @param {doramcap.provenance.v1.Step.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Step = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Step producer.
                 * @member {string} producer
                 * @memberof doramcap.provenance.v1.Step
                 * @instance
                 */
                Step.prototype.producer = "";

                /**
                 * Step name.
                 * @member {string} name
                 * @memberof doramcap.provenance.v1.Step
                 * @instance
                 */
                Step.prototype.name = "";

                /**
                 * Encodes the specified Step message. Does not implicitly {@link doramcap.provenance.v1.Step.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance.v1.Step
                 * @static
                 * @param {doramcap.provenance.v1.Step.$Properties} message Step message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Step.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.producer != null && $Object.hasOwnProperty.call(message, "producer") && message.producer !== "")
                        writer.uint32(/* id 1, wireType 2 =*/10).string(message.producer);
                    if (message.name != null && $Object.hasOwnProperty.call(message, "name") && message.name !== "")
                        writer.uint32(/* id 2, wireType 2 =*/18).string(message.name);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Step message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance.v1.Step
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.Step & doramcap.provenance.v1.Step.$Shape} Step
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Step.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance.v1.Step();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.producer = value;
                                else
                                    delete message.producer;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.name = value;
                                else
                                    delete message.name;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a Step message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance.v1.Step
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance.v1.Step} Step
                 */
                Step.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance.v1.Step)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance.v1.Step: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance.v1.Step();
                    if (object.producer != null)
                        if (typeof object.producer !== "string" || object.producer.length)
                            message.producer = $String(object.producer);
                    if (object.name != null)
                        if (typeof object.name !== "string" || object.name.length)
                            message.name = $String(object.name);
                    return message;
                };

                /**
                 * Creates a plain object from a Step message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance.v1.Step
                 * @static
                 * @param {doramcap.provenance.v1.Step} message Step
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Step.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.producer = "";
                        object.name = "";
                    }
                    if (message.producer != null && $Object.hasOwnProperty.call(message, "producer"))
                        object.producer = message.producer;
                    if (message.name != null && $Object.hasOwnProperty.call(message, "name"))
                        object.name = message.name;
                    return object;
                };

                /**
                 * Converts this Step to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance.v1.Step
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Step.prototype.toJSON = function() {
                    return Step.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Step
                 * @function getTypeUrl
                 * @memberof doramcap.provenance.v1.Step
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Step.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance.v1.Step";
                };

                return Step;
            })();

            v1.GenerationRef = (function() {

                /**
                 * Properties of a GenerationRef.
                 * @typedef {Object} doramcap.provenance.v1.GenerationRef.$Properties
                 * @property {string|null} [producer] GenerationRef producer
                 * @property {string|null} [epoch] GenerationRef epoch
                 * @property {string|null} [step] GenerationRef step
                 * @property {string|null} [generation] GenerationRef generation
                 * @property {number|Long|null} [sequence] GenerationRef sequence
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a GenerationRef.
                 * @memberof doramcap.provenance.v1
                 * @interface IGenerationRef
                 * @augments doramcap.provenance.v1.GenerationRef.$Properties
                 * @deprecated Use doramcap.provenance.v1.GenerationRef.$Properties instead.
                 */

                /**
                 * Shape of a GenerationRef.
                 * @typedef {doramcap.provenance.v1.GenerationRef.$Properties} doramcap.provenance.v1.GenerationRef.$Shape
                 */

                /**
                 * Constructs a new GenerationRef.
                 * @memberof doramcap.provenance.v1
                 * @classdesc Represents a GenerationRef.
                 * @constructor
                 * @param {doramcap.provenance.v1.GenerationRef.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const GenerationRef = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * GenerationRef producer.
                 * @member {string} producer
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @instance
                 */
                GenerationRef.prototype.producer = "";

                /**
                 * GenerationRef epoch.
                 * @member {string} epoch
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @instance
                 */
                GenerationRef.prototype.epoch = "";

                /**
                 * GenerationRef step.
                 * @member {string} step
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @instance
                 */
                GenerationRef.prototype.step = "";

                /**
                 * GenerationRef generation.
                 * @member {string} generation
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @instance
                 */
                GenerationRef.prototype.generation = "";

                /**
                 * GenerationRef sequence.
                 * @member {number|Long} sequence
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @instance
                 */
                GenerationRef.prototype.sequence = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified GenerationRef message. Does not implicitly {@link doramcap.provenance.v1.GenerationRef.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @static
                 * @param {doramcap.provenance.v1.GenerationRef.$Properties} message GenerationRef message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                GenerationRef.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.producer != null && $Object.hasOwnProperty.call(message, "producer") && message.producer !== "")
                        writer.uint32(/* id 1, wireType 2 =*/10).string(message.producer);
                    if (message.epoch != null && $Object.hasOwnProperty.call(message, "epoch") && message.epoch !== "")
                        writer.uint32(/* id 2, wireType 2 =*/18).string(message.epoch);
                    if (message.step != null && $Object.hasOwnProperty.call(message, "step") && message.step !== "")
                        writer.uint32(/* id 3, wireType 2 =*/26).string(message.step);
                    if (message.generation != null && $Object.hasOwnProperty.call(message, "generation") && message.generation !== "")
                        writer.uint32(/* id 4, wireType 2 =*/34).string(message.generation);
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence") && (typeof message.sequence === "object" ? message.sequence.low || message.sequence.high : message.sequence !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.sequence);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a GenerationRef message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.GenerationRef & doramcap.provenance.v1.GenerationRef.$Shape} GenerationRef
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                GenerationRef.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance.v1.GenerationRef();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.producer = value;
                                else
                                    delete message.producer;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.epoch = value;
                                else
                                    delete message.epoch;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.step = value;
                                else
                                    delete message.step;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.generation = value;
                                else
                                    delete message.generation;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.sequence = value;
                                else
                                    delete message.sequence;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a GenerationRef message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance.v1.GenerationRef} GenerationRef
                 */
                GenerationRef.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance.v1.GenerationRef)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance.v1.GenerationRef: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance.v1.GenerationRef();
                    if (object.producer != null)
                        if (typeof object.producer !== "string" || object.producer.length)
                            message.producer = $String(object.producer);
                    if (object.epoch != null)
                        if (typeof object.epoch !== "string" || object.epoch.length)
                            message.epoch = $String(object.epoch);
                    if (object.step != null)
                        if (typeof object.step !== "string" || object.step.length)
                            message.step = $String(object.step);
                    if (object.generation != null)
                        if (typeof object.generation !== "string" || object.generation.length)
                            message.generation = $String(object.generation);
                    if (object.sequence != null)
                        if (typeof object.sequence === "object" ? object.sequence.low || object.sequence.high : $Number(object.sequence) !== 0)
                            if ($util.Long)
                                message.sequence = $util.Long.fromValue(object.sequence, true);
                            else if (typeof object.sequence === "string")
                                message.sequence = $parseInt(object.sequence, 10);
                            else if (typeof object.sequence === "number")
                                message.sequence = object.sequence;
                            else if (typeof object.sequence === "object")
                                message.sequence = new $util.LongBits(object.sequence.low >>> 0, object.sequence.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a GenerationRef message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @static
                 * @param {doramcap.provenance.v1.GenerationRef} message GenerationRef
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                GenerationRef.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.producer = "";
                        object.epoch = "";
                        object.step = "";
                        object.generation = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.sequence = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.sequence = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.producer != null && $Object.hasOwnProperty.call(message, "producer"))
                        object.producer = message.producer;
                    if (message.epoch != null && $Object.hasOwnProperty.call(message, "epoch"))
                        object.epoch = message.epoch;
                    if (message.step != null && $Object.hasOwnProperty.call(message, "step"))
                        object.step = message.step;
                    if (message.generation != null && $Object.hasOwnProperty.call(message, "generation"))
                        object.generation = message.generation;
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.sequence = typeof message.sequence === "number" ? $BigInt(message.sequence) : $util.Long.fromBits(message.sequence.low >>> 0, message.sequence.high >>> 0, true).toBigInt();
                        else if (typeof message.sequence === "number")
                            object.sequence = options.longs === $String ? $String(message.sequence) : message.sequence;
                        else
                            object.sequence = options.longs === $String ? $util.Long.prototype.toString.call(message.sequence) : options.longs === $Number ? new $util.LongBits(message.sequence.low >>> 0, message.sequence.high >>> 0).toNumber(true) : message.sequence;
                    return object;
                };

                /**
                 * Converts this GenerationRef to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                GenerationRef.prototype.toJSON = function() {
                    return GenerationRef.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for GenerationRef
                 * @function getTypeUrl
                 * @memberof doramcap.provenance.v1.GenerationRef
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                GenerationRef.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance.v1.GenerationRef";
                };

                return GenerationRef;
            })();

            v1.Inputs = (function() {

                /**
                 * Properties of an Inputs.
                 * @typedef {Object} doramcap.provenance.v1.Inputs.$Properties
                 * @property {Array.<doramcap.provenance.v1.GenerationRef.$Properties>|null} [refs] Inputs refs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of an Inputs.
                 * @memberof doramcap.provenance.v1
                 * @interface IInputs
                 * @augments doramcap.provenance.v1.Inputs.$Properties
                 * @deprecated Use doramcap.provenance.v1.Inputs.$Properties instead.
                 */

                /**
                 * Shape of an Inputs.
                 * @typedef {doramcap.provenance.v1.Inputs.$Properties} doramcap.provenance.v1.Inputs.$Shape
                 */

                /**
                 * Constructs a new Inputs.
                 * @memberof doramcap.provenance.v1
                 * @classdesc Represents an Inputs.
                 * @constructor
                 * @param {doramcap.provenance.v1.Inputs.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Inputs = function (properties) {
                    this.refs = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Inputs refs.
                 * @member {Array.<doramcap.provenance.v1.GenerationRef.$Properties>} refs
                 * @memberof doramcap.provenance.v1.Inputs
                 * @instance
                 */
                Inputs.prototype.refs = $util.emptyArray;

                /**
                 * Encodes the specified Inputs message. Does not implicitly {@link doramcap.provenance.v1.Inputs.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance.v1.Inputs
                 * @static
                 * @param {doramcap.provenance.v1.Inputs.$Properties} message Inputs message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Inputs.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.refs != null && message.refs.length)
                        for (let i = 0; i < message.refs.length; ++i)
                            $root.doramcap.provenance.v1.GenerationRef.encode(message.refs[i], writer.uint32(/* id 1, wireType 2 =*/10).fork(), _depth + 1).ldelim();
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes an Inputs message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance.v1.Inputs
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.Inputs & doramcap.provenance.v1.Inputs.$Shape} Inputs
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Inputs.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance.v1.Inputs();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.refs && message.refs.length))
                                    message.refs = [];
                                message.refs.push($root.doramcap.provenance.v1.GenerationRef.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates an Inputs message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance.v1.Inputs
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance.v1.Inputs} Inputs
                 */
                Inputs.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance.v1.Inputs)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance.v1.Inputs: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance.v1.Inputs();
                    if (object.refs) {
                        if (!$Array.isArray(object.refs))
                            throw $TypeError(".doramcap.provenance.v1.Inputs.refs: array expected");
                        message.refs = $Array(object.refs.length);
                        for (let i = 0; i < object.refs.length; ++i) {
                            if (!$util.isObject(object.refs[i]))
                                throw $TypeError(".doramcap.provenance.v1.Inputs.refs: object expected");
                            message.refs[i] = $root.doramcap.provenance.v1.GenerationRef.fromObject(object.refs[i], _depth + 1);
                        }
                    }
                    return message;
                };

                /**
                 * Creates a plain object from an Inputs message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance.v1.Inputs
                 * @static
                 * @param {doramcap.provenance.v1.Inputs} message Inputs
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Inputs.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.arrays || options.defaults)
                        object.refs = [];
                    if (message.refs && message.refs.length) {
                        object.refs = $Array(message.refs.length);
                        for (let j = 0; j < message.refs.length; ++j)
                            object.refs[j] = $root.doramcap.provenance.v1.GenerationRef.toObject(message.refs[j], options, _depth + 1);
                    }
                    return object;
                };

                /**
                 * Converts this Inputs to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance.v1.Inputs
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Inputs.prototype.toJSON = function() {
                    return Inputs.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Inputs
                 * @function getTypeUrl
                 * @memberof doramcap.provenance.v1.Inputs
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Inputs.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance.v1.Inputs";
                };

                return Inputs;
            })();

            v1.StepUpdate = (function() {

                /**
                 * Properties of a StepUpdate.
                 * @typedef {Object} doramcap.provenance.v1.StepUpdate.$Properties
                 * @property {number|null} [version] StepUpdate version
                 * @property {doramcap.provenance.v1.GenerationRef.$Properties|null} [generation] StepUpdate generation
                 * @property {string|null} [clockDomain] StepUpdate clockDomain
                 * @property {string|null} [invocation] StepUpdate invocation
                 * @property {number|Long|null} [startNs] StepUpdate startNs
                 * @property {number|Long|null} [endNs] StepUpdate endNs
                 * @property {doramcap.provenance.v1.Inputs.$Properties|null} [inputs] StepUpdate inputs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a StepUpdate.
                 * @memberof doramcap.provenance.v1
                 * @interface IStepUpdate
                 * @augments doramcap.provenance.v1.StepUpdate.$Properties
                 * @deprecated Use doramcap.provenance.v1.StepUpdate.$Properties instead.
                 */

                /**
                 * Shape of a StepUpdate.
                 * @typedef {doramcap.provenance.v1.StepUpdate.$Properties} doramcap.provenance.v1.StepUpdate.$Shape
                 */

                /**
                 * Constructs a new StepUpdate.
                 * @memberof doramcap.provenance.v1
                 * @classdesc Represents a StepUpdate.
                 * @constructor
                 * @param {doramcap.provenance.v1.StepUpdate.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const StepUpdate = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * StepUpdate version.
                 * @member {number} version
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @instance
                 */
                StepUpdate.prototype.version = 0;

                /**
                 * StepUpdate generation.
                 * @member {doramcap.provenance.v1.GenerationRef.$Properties|null|undefined} generation
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @instance
                 */
                StepUpdate.prototype.generation = null;

                /**
                 * StepUpdate clockDomain.
                 * @member {string} clockDomain
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @instance
                 */
                StepUpdate.prototype.clockDomain = "";

                /**
                 * StepUpdate invocation.
                 * @member {string} invocation
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @instance
                 */
                StepUpdate.prototype.invocation = "";

                /**
                 * StepUpdate startNs.
                 * @member {number|Long|null|undefined} startNs
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @instance
                 */
                StepUpdate.prototype.startNs = null;

                /**
                 * StepUpdate endNs.
                 * @member {number|Long|null|undefined} endNs
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @instance
                 */
                StepUpdate.prototype.endNs = null;

                /**
                 * StepUpdate inputs.
                 * @member {doramcap.provenance.v1.Inputs.$Properties|null|undefined} inputs
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @instance
                 */
                StepUpdate.prototype.inputs = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(StepUpdate.prototype, "_startNs", {
                    get: $util.oneOfGetter($oneOfFields = ["startNs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(StepUpdate.prototype, "_endNs", {
                    get: $util.oneOfGetter($oneOfFields = ["endNs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified StepUpdate message. Does not implicitly {@link doramcap.provenance.v1.StepUpdate.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @static
                 * @param {doramcap.provenance.v1.StepUpdate.$Properties} message StepUpdate message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                StepUpdate.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version") && message.version !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.version);
                    if (message.generation != null && $Object.hasOwnProperty.call(message, "generation"))
                        $root.doramcap.provenance.v1.GenerationRef.encode(message.generation, writer.uint32(/* id 2, wireType 2 =*/18).fork(), _depth + 1).ldelim();
                    if (message.clockDomain != null && $Object.hasOwnProperty.call(message, "clockDomain") && message.clockDomain !== "")
                        writer.uint32(/* id 3, wireType 2 =*/26).string(message.clockDomain);
                    if (message.invocation != null && $Object.hasOwnProperty.call(message, "invocation") && message.invocation !== "")
                        writer.uint32(/* id 4, wireType 2 =*/34).string(message.invocation);
                    if (message.startNs != null && $Object.hasOwnProperty.call(message, "startNs"))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.startNs);
                    if (message.endNs != null && $Object.hasOwnProperty.call(message, "endNs"))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.endNs);
                    if (message.inputs != null && $Object.hasOwnProperty.call(message, "inputs"))
                        $root.doramcap.provenance.v1.Inputs.encode(message.inputs, writer.uint32(/* id 7, wireType 2 =*/58).fork(), _depth + 1).ldelim();
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a StepUpdate message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.StepUpdate & doramcap.provenance.v1.StepUpdate.$Shape} StepUpdate
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                StepUpdate.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance.v1.StepUpdate();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.version = value;
                                else
                                    delete message.version;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 2)
                                    break;
                                message.generation = $root.doramcap.provenance.v1.GenerationRef.decode(reader, reader.size(), $undefined, _depth + 1, message.generation);
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.clockDomain = value;
                                else
                                    delete message.clockDomain;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.invocation = value;
                                else
                                    delete message.invocation;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                message.startNs = reader.uint64();
                                message._startNs = "startNs";
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                message.endNs = reader.uint64();
                                message._endNs = "endNs";
                                continue;
                            }
                        case 7: {
                                if (wireType !== 2)
                                    break;
                                message.inputs = $root.doramcap.provenance.v1.Inputs.decode(reader, reader.size(), $undefined, _depth + 1, message.inputs);
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a StepUpdate message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance.v1.StepUpdate} StepUpdate
                 */
                StepUpdate.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance.v1.StepUpdate)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance.v1.StepUpdate: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance.v1.StepUpdate();
                    if (object.version != null)
                        if ($Number(object.version) !== 0)
                            message.version = object.version >>> 0;
                    if (object.generation != null) {
                        if (!$util.isObject(object.generation))
                            throw $TypeError(".doramcap.provenance.v1.StepUpdate.generation: object expected");
                        message.generation = $root.doramcap.provenance.v1.GenerationRef.fromObject(object.generation, _depth + 1);
                    }
                    if (object.clockDomain != null)
                        if (typeof object.clockDomain !== "string" || object.clockDomain.length)
                            message.clockDomain = $String(object.clockDomain);
                    if (object.invocation != null)
                        if (typeof object.invocation !== "string" || object.invocation.length)
                            message.invocation = $String(object.invocation);
                    if (object.startNs != null)
                        if ($util.Long)
                            message.startNs = $util.Long.fromValue(object.startNs, true);
                        else if (typeof object.startNs === "string")
                            message.startNs = $parseInt(object.startNs, 10);
                        else if (typeof object.startNs === "number")
                            message.startNs = object.startNs;
                        else if (typeof object.startNs === "object")
                            message.startNs = new $util.LongBits(object.startNs.low >>> 0, object.startNs.high >>> 0).toNumber(true);
                    if (object.endNs != null)
                        if ($util.Long)
                            message.endNs = $util.Long.fromValue(object.endNs, true);
                        else if (typeof object.endNs === "string")
                            message.endNs = $parseInt(object.endNs, 10);
                        else if (typeof object.endNs === "number")
                            message.endNs = object.endNs;
                        else if (typeof object.endNs === "object")
                            message.endNs = new $util.LongBits(object.endNs.low >>> 0, object.endNs.high >>> 0).toNumber(true);
                    if (object.inputs != null) {
                        if (!$util.isObject(object.inputs))
                            throw $TypeError(".doramcap.provenance.v1.StepUpdate.inputs: object expected");
                        message.inputs = $root.doramcap.provenance.v1.Inputs.fromObject(object.inputs, _depth + 1);
                    }
                    return message;
                };

                /**
                 * Creates a plain object from a StepUpdate message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @static
                 * @param {doramcap.provenance.v1.StepUpdate} message StepUpdate
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                StepUpdate.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.version = 0;
                        object.generation = null;
                        object.clockDomain = "";
                        object.invocation = "";
                        object.inputs = null;
                    }
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version"))
                        object.version = message.version;
                    if (message.generation != null && $Object.hasOwnProperty.call(message, "generation"))
                        object.generation = $root.doramcap.provenance.v1.GenerationRef.toObject(message.generation, options, _depth + 1);
                    if (message.clockDomain != null && $Object.hasOwnProperty.call(message, "clockDomain"))
                        object.clockDomain = message.clockDomain;
                    if (message.invocation != null && $Object.hasOwnProperty.call(message, "invocation"))
                        object.invocation = message.invocation;
                    if (message.startNs != null && $Object.hasOwnProperty.call(message, "startNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.startNs = typeof message.startNs === "number" ? $BigInt(message.startNs) : $util.Long.fromBits(message.startNs.low >>> 0, message.startNs.high >>> 0, true).toBigInt();
                        else if (typeof message.startNs === "number")
                            object.startNs = options.longs === $String ? $String(message.startNs) : message.startNs;
                        else
                            object.startNs = options.longs === $String ? $util.Long.prototype.toString.call(message.startNs) : options.longs === $Number ? new $util.LongBits(message.startNs.low >>> 0, message.startNs.high >>> 0).toNumber(true) : message.startNs;
                    if (message.endNs != null && $Object.hasOwnProperty.call(message, "endNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.endNs = typeof message.endNs === "number" ? $BigInt(message.endNs) : $util.Long.fromBits(message.endNs.low >>> 0, message.endNs.high >>> 0, true).toBigInt();
                        else if (typeof message.endNs === "number")
                            object.endNs = options.longs === $String ? $String(message.endNs) : message.endNs;
                        else
                            object.endNs = options.longs === $String ? $util.Long.prototype.toString.call(message.endNs) : options.longs === $Number ? new $util.LongBits(message.endNs.low >>> 0, message.endNs.high >>> 0).toNumber(true) : message.endNs;
                    if (message.inputs != null && $Object.hasOwnProperty.call(message, "inputs"))
                        object.inputs = $root.doramcap.provenance.v1.Inputs.toObject(message.inputs, options, _depth + 1);
                    return object;
                };

                /**
                 * Converts this StepUpdate to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                StepUpdate.prototype.toJSON = function() {
                    return StepUpdate.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for StepUpdate
                 * @function getTypeUrl
                 * @memberof doramcap.provenance.v1.StepUpdate
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                StepUpdate.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance.v1.StepUpdate";
                };

                return StepUpdate;
            })();

            /**
             * Boundary enum.
             * @name doramcap.provenance.v1.Boundary
             * @enum {number}
             * @property {number} BOUNDARY_UNSPECIFIED=0 BOUNDARY_UNSPECIFIED value
             * @property {number} START=1 START value
             * @property {number} END=2 END value
             */
            v1.Boundary = (function() {
                const valuesById = $Object.create(null), values = $Object.create(valuesById);
                values[valuesById[0] = "BOUNDARY_UNSPECIFIED"] = 0;
                values[valuesById[1] = "START"] = 1;
                values[valuesById[2] = "END"] = 2;
                return values;
            })();

            /**
             * Selector enum.
             * @name doramcap.provenance.v1.Selector
             * @enum {number}
             * @property {number} SELECTOR_UNSPECIFIED=0 SELECTOR_UNSPECIFIED value
             * @property {number} CONSUMED_ANCESTOR=1 CONSUMED_ANCESTOR value
             * @property {number} SAME_INVOCATION=2 SAME_INVOCATION value
             * @property {number} PREVIOUS=3 PREVIOUS value
             * @property {number} LATEST_KNOWN_AT_ANCHOR=4 LATEST_KNOWN_AT_ANCHOR value
             */
            v1.Selector = (function() {
                const valuesById = $Object.create(null), values = $Object.create(valuesById);
                values[valuesById[0] = "SELECTOR_UNSPECIFIED"] = 0;
                values[valuesById[1] = "CONSUMED_ANCESTOR"] = 1;
                values[valuesById[2] = "SAME_INVOCATION"] = 2;
                values[valuesById[3] = "PREVIOUS"] = 3;
                values[valuesById[4] = "LATEST_KNOWN_AT_ANCHOR"] = 4;
                return values;
            })();

            v1.Probe = (function() {

                /**
                 * Properties of a Probe.
                 * @typedef {Object} doramcap.provenance.v1.Probe.$Properties
                 * @property {doramcap.provenance.v1.Step.$Properties|null} [step] Probe step
                 * @property {doramcap.provenance.v1.Boundary|null} [boundary] Probe boundary
                 * @property {doramcap.provenance.v1.Selector|null} [selector] Probe selector
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Probe.
                 * @memberof doramcap.provenance.v1
                 * @interface IProbe
                 * @augments doramcap.provenance.v1.Probe.$Properties
                 * @deprecated Use doramcap.provenance.v1.Probe.$Properties instead.
                 */

                /**
                 * Shape of a Probe.
                 * @typedef {doramcap.provenance.v1.Probe.$Properties} doramcap.provenance.v1.Probe.$Shape
                 */

                /**
                 * Constructs a new Probe.
                 * @memberof doramcap.provenance.v1
                 * @classdesc Represents a Probe.
                 * @constructor
                 * @param {doramcap.provenance.v1.Probe.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Probe = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Probe step.
                 * @member {doramcap.provenance.v1.Step.$Properties|null|undefined} step
                 * @memberof doramcap.provenance.v1.Probe
                 * @instance
                 */
                Probe.prototype.step = null;

                /**
                 * Probe boundary.
                 * @member {doramcap.provenance.v1.Boundary} boundary
                 * @memberof doramcap.provenance.v1.Probe
                 * @instance
                 */
                Probe.prototype.boundary = 0;

                /**
                 * Probe selector.
                 * @member {doramcap.provenance.v1.Selector} selector
                 * @memberof doramcap.provenance.v1.Probe
                 * @instance
                 */
                Probe.prototype.selector = 0;

                /**
                 * Encodes the specified Probe message. Does not implicitly {@link doramcap.provenance.v1.Probe.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance.v1.Probe
                 * @static
                 * @param {doramcap.provenance.v1.Probe.$Properties} message Probe message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Probe.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.step != null && $Object.hasOwnProperty.call(message, "step"))
                        $root.doramcap.provenance.v1.Step.encode(message.step, writer.uint32(/* id 1, wireType 2 =*/10).fork(), _depth + 1).ldelim();
                    if (message.boundary != null && $Object.hasOwnProperty.call(message, "boundary") && message.boundary !== 0)
                        writer.uint32(/* id 2, wireType 0 =*/16).int32(message.boundary);
                    if (message.selector != null && $Object.hasOwnProperty.call(message, "selector") && message.selector !== 0)
                        writer.uint32(/* id 3, wireType 0 =*/24).int32(message.selector);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Probe message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance.v1.Probe
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.Probe & doramcap.provenance.v1.Probe.$Shape} Probe
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Probe.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance.v1.Probe();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 2)
                                    break;
                                message.step = $root.doramcap.provenance.v1.Step.decode(reader, reader.size(), $undefined, _depth + 1, message.step);
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.int32())
                                    message.boundary = value;
                                else
                                    delete message.boundary;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.int32())
                                    message.selector = value;
                                else
                                    delete message.selector;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a Probe message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance.v1.Probe
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance.v1.Probe} Probe
                 */
                Probe.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance.v1.Probe)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance.v1.Probe: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance.v1.Probe();
                    if (object.step != null) {
                        if (!$util.isObject(object.step))
                            throw $TypeError(".doramcap.provenance.v1.Probe.step: object expected");
                        message.step = $root.doramcap.provenance.v1.Step.fromObject(object.step, _depth + 1);
                    }
                    if (object.boundary !== 0 && (typeof object.boundary !== "string" || $root.doramcap.provenance.v1.Boundary[object.boundary] !== 0))
                        switch (object.boundary) {
                        case "BOUNDARY_UNSPECIFIED":
                        case 0:
                            message.boundary = 0;
                            break;
                        case "START":
                        case 1:
                            message.boundary = 1;
                            break;
                        case "END":
                        case 2:
                            message.boundary = 2;
                            break;
                        default:
                            if (typeof object.boundary === "number" && (object.boundary | 0) === object.boundary)
                                message.boundary = object.boundary;
                        }
                    if (object.selector !== 0 && (typeof object.selector !== "string" || $root.doramcap.provenance.v1.Selector[object.selector] !== 0))
                        switch (object.selector) {
                        case "SELECTOR_UNSPECIFIED":
                        case 0:
                            message.selector = 0;
                            break;
                        case "CONSUMED_ANCESTOR":
                        case 1:
                            message.selector = 1;
                            break;
                        case "SAME_INVOCATION":
                        case 2:
                            message.selector = 2;
                            break;
                        case "PREVIOUS":
                        case 3:
                            message.selector = 3;
                            break;
                        case "LATEST_KNOWN_AT_ANCHOR":
                        case 4:
                            message.selector = 4;
                            break;
                        default:
                            if (typeof object.selector === "number" && (object.selector | 0) === object.selector)
                                message.selector = object.selector;
                        }
                    return message;
                };

                /**
                 * Creates a plain object from a Probe message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance.v1.Probe
                 * @static
                 * @param {doramcap.provenance.v1.Probe} message Probe
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Probe.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.step = null;
                        object.boundary = options.enums === $String ? "BOUNDARY_UNSPECIFIED" : 0;
                        object.selector = options.enums === $String ? "SELECTOR_UNSPECIFIED" : 0;
                    }
                    if (message.step != null && $Object.hasOwnProperty.call(message, "step"))
                        object.step = $root.doramcap.provenance.v1.Step.toObject(message.step, options, _depth + 1);
                    if (message.boundary != null && $Object.hasOwnProperty.call(message, "boundary"))
                        object.boundary = options.enums === $String ? $root.doramcap.provenance.v1.Boundary[message.boundary] === $undefined ? message.boundary : $root.doramcap.provenance.v1.Boundary[message.boundary] : message.boundary;
                    if (message.selector != null && $Object.hasOwnProperty.call(message, "selector"))
                        object.selector = options.enums === $String ? $root.doramcap.provenance.v1.Selector[message.selector] === $undefined ? message.selector : $root.doramcap.provenance.v1.Selector[message.selector] : message.selector;
                    return object;
                };

                /**
                 * Converts this Probe to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance.v1.Probe
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Probe.prototype.toJSON = function() {
                    return Probe.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Probe
                 * @function getTypeUrl
                 * @memberof doramcap.provenance.v1.Probe
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Probe.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance.v1.Probe";
                };

                return Probe;
            })();

            /**
             * MissingDataPolicy enum.
             * @name doramcap.provenance.v1.MissingDataPolicy
             * @enum {number}
             * @property {number} MISSING_DATA_POLICY_UNSPECIFIED=0 MISSING_DATA_POLICY_UNSPECIFIED value
             * @property {number} WAIT_UNTIL_EVICTION=1 WAIT_UNTIL_EVICTION value
             */
            v1.MissingDataPolicy = (function() {
                const valuesById = $Object.create(null), values = $Object.create(valuesById);
                values[valuesById[0] = "MISSING_DATA_POLICY_UNSPECIFIED"] = 0;
                values[valuesById[1] = "WAIT_UNTIL_EVICTION"] = 1;
                return values;
            })();

            v1.MeasurementDefinition = (function() {

                /**
                 * Properties of a MeasurementDefinition.
                 * @typedef {Object} doramcap.provenance.v1.MeasurementDefinition.$Properties
                 * @property {string|null} [name] MeasurementDefinition name
                 * @property {doramcap.provenance.v1.Step.$Properties|null} [anchor] MeasurementDefinition anchor
                 * @property {doramcap.provenance.v1.Probe.$Properties|null} [start] MeasurementDefinition start
                 * @property {doramcap.provenance.v1.Probe.$Properties|null} [end] MeasurementDefinition end
                 * @property {doramcap.provenance.v1.MissingDataPolicy|null} [missingData] MeasurementDefinition missingData
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a MeasurementDefinition.
                 * @memberof doramcap.provenance.v1
                 * @interface IMeasurementDefinition
                 * @augments doramcap.provenance.v1.MeasurementDefinition.$Properties
                 * @deprecated Use doramcap.provenance.v1.MeasurementDefinition.$Properties instead.
                 */

                /**
                 * Shape of a MeasurementDefinition.
                 * @typedef {doramcap.provenance.v1.MeasurementDefinition.$Properties} doramcap.provenance.v1.MeasurementDefinition.$Shape
                 */

                /**
                 * Constructs a new MeasurementDefinition.
                 * @memberof doramcap.provenance.v1
                 * @classdesc Represents a MeasurementDefinition.
                 * @constructor
                 * @param {doramcap.provenance.v1.MeasurementDefinition.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const MeasurementDefinition = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * MeasurementDefinition name.
                 * @member {string} name
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @instance
                 */
                MeasurementDefinition.prototype.name = "";

                /**
                 * MeasurementDefinition anchor.
                 * @member {doramcap.provenance.v1.Step.$Properties|null|undefined} anchor
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @instance
                 */
                MeasurementDefinition.prototype.anchor = null;

                /**
                 * MeasurementDefinition start.
                 * @member {doramcap.provenance.v1.Probe.$Properties|null|undefined} start
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @instance
                 */
                MeasurementDefinition.prototype.start = null;

                /**
                 * MeasurementDefinition end.
                 * @member {doramcap.provenance.v1.Probe.$Properties|null|undefined} end
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @instance
                 */
                MeasurementDefinition.prototype.end = null;

                /**
                 * MeasurementDefinition missingData.
                 * @member {doramcap.provenance.v1.MissingDataPolicy} missingData
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @instance
                 */
                MeasurementDefinition.prototype.missingData = 0;

                /**
                 * Encodes the specified MeasurementDefinition message. Does not implicitly {@link doramcap.provenance.v1.MeasurementDefinition.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @static
                 * @param {doramcap.provenance.v1.MeasurementDefinition.$Properties} message MeasurementDefinition message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                MeasurementDefinition.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.name != null && $Object.hasOwnProperty.call(message, "name") && message.name !== "")
                        writer.uint32(/* id 1, wireType 2 =*/10).string(message.name);
                    if (message.anchor != null && $Object.hasOwnProperty.call(message, "anchor"))
                        $root.doramcap.provenance.v1.Step.encode(message.anchor, writer.uint32(/* id 2, wireType 2 =*/18).fork(), _depth + 1).ldelim();
                    if (message.start != null && $Object.hasOwnProperty.call(message, "start"))
                        $root.doramcap.provenance.v1.Probe.encode(message.start, writer.uint32(/* id 3, wireType 2 =*/26).fork(), _depth + 1).ldelim();
                    if (message.end != null && $Object.hasOwnProperty.call(message, "end"))
                        $root.doramcap.provenance.v1.Probe.encode(message.end, writer.uint32(/* id 4, wireType 2 =*/34).fork(), _depth + 1).ldelim();
                    if (message.missingData != null && $Object.hasOwnProperty.call(message, "missingData") && message.missingData !== 0)
                        writer.uint32(/* id 5, wireType 0 =*/40).int32(message.missingData);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a MeasurementDefinition message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.MeasurementDefinition & doramcap.provenance.v1.MeasurementDefinition.$Shape} MeasurementDefinition
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                MeasurementDefinition.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance.v1.MeasurementDefinition();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.name = value;
                                else
                                    delete message.name;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 2)
                                    break;
                                message.anchor = $root.doramcap.provenance.v1.Step.decode(reader, reader.size(), $undefined, _depth + 1, message.anchor);
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                message.start = $root.doramcap.provenance.v1.Probe.decode(reader, reader.size(), $undefined, _depth + 1, message.start);
                                continue;
                            }
                        case 4: {
                                if (wireType !== 2)
                                    break;
                                message.end = $root.doramcap.provenance.v1.Probe.decode(reader, reader.size(), $undefined, _depth + 1, message.end);
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.int32())
                                    message.missingData = value;
                                else
                                    delete message.missingData;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a MeasurementDefinition message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance.v1.MeasurementDefinition} MeasurementDefinition
                 */
                MeasurementDefinition.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance.v1.MeasurementDefinition)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance.v1.MeasurementDefinition: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance.v1.MeasurementDefinition();
                    if (object.name != null)
                        if (typeof object.name !== "string" || object.name.length)
                            message.name = $String(object.name);
                    if (object.anchor != null) {
                        if (!$util.isObject(object.anchor))
                            throw $TypeError(".doramcap.provenance.v1.MeasurementDefinition.anchor: object expected");
                        message.anchor = $root.doramcap.provenance.v1.Step.fromObject(object.anchor, _depth + 1);
                    }
                    if (object.start != null) {
                        if (!$util.isObject(object.start))
                            throw $TypeError(".doramcap.provenance.v1.MeasurementDefinition.start: object expected");
                        message.start = $root.doramcap.provenance.v1.Probe.fromObject(object.start, _depth + 1);
                    }
                    if (object.end != null) {
                        if (!$util.isObject(object.end))
                            throw $TypeError(".doramcap.provenance.v1.MeasurementDefinition.end: object expected");
                        message.end = $root.doramcap.provenance.v1.Probe.fromObject(object.end, _depth + 1);
                    }
                    if (object.missingData !== 0 && (typeof object.missingData !== "string" || $root.doramcap.provenance.v1.MissingDataPolicy[object.missingData] !== 0))
                        switch (object.missingData) {
                        case "MISSING_DATA_POLICY_UNSPECIFIED":
                        case 0:
                            message.missingData = 0;
                            break;
                        case "WAIT_UNTIL_EVICTION":
                        case 1:
                            message.missingData = 1;
                            break;
                        default:
                            if (typeof object.missingData === "number" && (object.missingData | 0) === object.missingData)
                                message.missingData = object.missingData;
                        }
                    return message;
                };

                /**
                 * Creates a plain object from a MeasurementDefinition message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @static
                 * @param {doramcap.provenance.v1.MeasurementDefinition} message MeasurementDefinition
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                MeasurementDefinition.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.name = "";
                        object.anchor = null;
                        object.start = null;
                        object.end = null;
                        object.missingData = options.enums === $String ? "MISSING_DATA_POLICY_UNSPECIFIED" : 0;
                    }
                    if (message.name != null && $Object.hasOwnProperty.call(message, "name"))
                        object.name = message.name;
                    if (message.anchor != null && $Object.hasOwnProperty.call(message, "anchor"))
                        object.anchor = $root.doramcap.provenance.v1.Step.toObject(message.anchor, options, _depth + 1);
                    if (message.start != null && $Object.hasOwnProperty.call(message, "start"))
                        object.start = $root.doramcap.provenance.v1.Probe.toObject(message.start, options, _depth + 1);
                    if (message.end != null && $Object.hasOwnProperty.call(message, "end"))
                        object.end = $root.doramcap.provenance.v1.Probe.toObject(message.end, options, _depth + 1);
                    if (message.missingData != null && $Object.hasOwnProperty.call(message, "missingData"))
                        object.missingData = options.enums === $String ? $root.doramcap.provenance.v1.MissingDataPolicy[message.missingData] === $undefined ? message.missingData : $root.doramcap.provenance.v1.MissingDataPolicy[message.missingData] : message.missingData;
                    return object;
                };

                /**
                 * Converts this MeasurementDefinition to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                MeasurementDefinition.prototype.toJSON = function() {
                    return MeasurementDefinition.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for MeasurementDefinition
                 * @function getTypeUrl
                 * @memberof doramcap.provenance.v1.MeasurementDefinition
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                MeasurementDefinition.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance.v1.MeasurementDefinition";
                };

                return MeasurementDefinition;
            })();

            v1.Definitions = (function() {

                /**
                 * Properties of a Definitions.
                 * @typedef {Object} doramcap.provenance.v1.Definitions.$Properties
                 * @property {number|null} [version] Definitions version
                 * @property {Array.<doramcap.provenance.v1.MeasurementDefinition.$Properties>|null} [measurements] Definitions measurements
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Definitions.
                 * @memberof doramcap.provenance.v1
                 * @interface IDefinitions
                 * @augments doramcap.provenance.v1.Definitions.$Properties
                 * @deprecated Use doramcap.provenance.v1.Definitions.$Properties instead.
                 */

                /**
                 * Shape of a Definitions.
                 * @typedef {doramcap.provenance.v1.Definitions.$Properties} doramcap.provenance.v1.Definitions.$Shape
                 */

                /**
                 * Constructs a new Definitions.
                 * @memberof doramcap.provenance.v1
                 * @classdesc Represents a Definitions.
                 * @constructor
                 * @param {doramcap.provenance.v1.Definitions.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Definitions = function (properties) {
                    this.measurements = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Definitions version.
                 * @member {number} version
                 * @memberof doramcap.provenance.v1.Definitions
                 * @instance
                 */
                Definitions.prototype.version = 0;

                /**
                 * Definitions measurements.
                 * @member {Array.<doramcap.provenance.v1.MeasurementDefinition.$Properties>} measurements
                 * @memberof doramcap.provenance.v1.Definitions
                 * @instance
                 */
                Definitions.prototype.measurements = $util.emptyArray;

                /**
                 * Encodes the specified Definitions message. Does not implicitly {@link doramcap.provenance.v1.Definitions.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance.v1.Definitions
                 * @static
                 * @param {doramcap.provenance.v1.Definitions.$Properties} message Definitions message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Definitions.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version") && message.version !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.version);
                    if (message.measurements != null && message.measurements.length)
                        for (let i = 0; i < message.measurements.length; ++i)
                            $root.doramcap.provenance.v1.MeasurementDefinition.encode(message.measurements[i], writer.uint32(/* id 2, wireType 2 =*/18).fork(), _depth + 1).ldelim();
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Definitions message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance.v1.Definitions
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.Definitions & doramcap.provenance.v1.Definitions.$Shape} Definitions
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Definitions.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance.v1.Definitions();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.version = value;
                                else
                                    delete message.version;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.measurements && message.measurements.length))
                                    message.measurements = [];
                                message.measurements.push($root.doramcap.provenance.v1.MeasurementDefinition.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a Definitions message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance.v1.Definitions
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance.v1.Definitions} Definitions
                 */
                Definitions.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance.v1.Definitions)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance.v1.Definitions: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance.v1.Definitions();
                    if (object.version != null)
                        if ($Number(object.version) !== 0)
                            message.version = object.version >>> 0;
                    if (object.measurements) {
                        if (!$Array.isArray(object.measurements))
                            throw $TypeError(".doramcap.provenance.v1.Definitions.measurements: array expected");
                        message.measurements = $Array(object.measurements.length);
                        for (let i = 0; i < object.measurements.length; ++i) {
                            if (!$util.isObject(object.measurements[i]))
                                throw $TypeError(".doramcap.provenance.v1.Definitions.measurements: object expected");
                            message.measurements[i] = $root.doramcap.provenance.v1.MeasurementDefinition.fromObject(object.measurements[i], _depth + 1);
                        }
                    }
                    return message;
                };

                /**
                 * Creates a plain object from a Definitions message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance.v1.Definitions
                 * @static
                 * @param {doramcap.provenance.v1.Definitions} message Definitions
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Definitions.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.arrays || options.defaults)
                        object.measurements = [];
                    if (options.defaults)
                        object.version = 0;
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version"))
                        object.version = message.version;
                    if (message.measurements && message.measurements.length) {
                        object.measurements = $Array(message.measurements.length);
                        for (let j = 0; j < message.measurements.length; ++j)
                            object.measurements[j] = $root.doramcap.provenance.v1.MeasurementDefinition.toObject(message.measurements[j], options, _depth + 1);
                    }
                    return object;
                };

                /**
                 * Converts this Definitions to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance.v1.Definitions
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Definitions.prototype.toJSON = function() {
                    return Definitions.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Definitions
                 * @function getTypeUrl
                 * @memberof doramcap.provenance.v1.Definitions
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Definitions.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance.v1.Definitions";
                };

                return Definitions;
            })();

            /**
             * Status enum.
             * @name doramcap.provenance.v1.Status
             * @enum {number}
             * @property {number} STATUS_UNSPECIFIED=0 STATUS_UNSPECIFIED value
             * @property {number} COMPLETE=1 COMPLETE value
             * @property {number} INCOMPLETE=2 INCOMPLETE value
             */
            v1.Status = (function() {
                const valuesById = $Object.create(null), values = $Object.create(valuesById);
                values[valuesById[0] = "STATUS_UNSPECIFIED"] = 0;
                values[valuesById[1] = "COMPLETE"] = 1;
                values[valuesById[2] = "INCOMPLETE"] = 2;
                return values;
            })();

            /**
             * Reason enum.
             * @name doramcap.provenance.v1.Reason
             * @enum {number}
             * @property {number} REASON_UNSPECIFIED=0 REASON_UNSPECIFIED value
             * @property {number} HISTORY_EVICTED=1 HISTORY_EVICTED value
             * @property {number} ANCHOR_EVICTED=2 ANCHOR_EVICTED value
             * @property {number} NO_MATCH=3 NO_MATCH value
             * @property {number} AMBIGUOUS=4 AMBIGUOUS value
             * @property {number} CLOCK_MISMATCH=5 CLOCK_MISMATCH value
             * @property {number} INVALID_DURATION=6 INVALID_DURATION value
             * @property {number} TRAVERSAL_LIMIT=7 TRAVERSAL_LIMIT value
             */
            v1.Reason = (function() {
                const valuesById = $Object.create(null), values = $Object.create(valuesById);
                values[valuesById[0] = "REASON_UNSPECIFIED"] = 0;
                values[valuesById[1] = "HISTORY_EVICTED"] = 1;
                values[valuesById[2] = "ANCHOR_EVICTED"] = 2;
                values[valuesById[3] = "NO_MATCH"] = 3;
                values[valuesById[4] = "AMBIGUOUS"] = 4;
                values[valuesById[5] = "CLOCK_MISMATCH"] = 5;
                values[valuesById[6] = "INVALID_DURATION"] = 6;
                values[valuesById[7] = "TRAVERSAL_LIMIT"] = 7;
                return values;
            })();

            v1.SelectedProbe = (function() {

                /**
                 * Properties of a SelectedProbe.
                 * @typedef {Object} doramcap.provenance.v1.SelectedProbe.$Properties
                 * @property {doramcap.provenance.v1.GenerationRef.$Properties|null} [generation] SelectedProbe generation
                 * @property {doramcap.provenance.v1.Boundary|null} [boundary] SelectedProbe boundary
                 * @property {number|Long|null} [timestampNs] SelectedProbe timestampNs
                 * @property {string|null} [clockDomain] SelectedProbe clockDomain
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a SelectedProbe.
                 * @memberof doramcap.provenance.v1
                 * @interface ISelectedProbe
                 * @augments doramcap.provenance.v1.SelectedProbe.$Properties
                 * @deprecated Use doramcap.provenance.v1.SelectedProbe.$Properties instead.
                 */

                /**
                 * Shape of a SelectedProbe.
                 * @typedef {doramcap.provenance.v1.SelectedProbe.$Properties} doramcap.provenance.v1.SelectedProbe.$Shape
                 */

                /**
                 * Constructs a new SelectedProbe.
                 * @memberof doramcap.provenance.v1
                 * @classdesc Represents a SelectedProbe.
                 * @constructor
                 * @param {doramcap.provenance.v1.SelectedProbe.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const SelectedProbe = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * SelectedProbe generation.
                 * @member {doramcap.provenance.v1.GenerationRef.$Properties|null|undefined} generation
                 * @memberof doramcap.provenance.v1.SelectedProbe
                 * @instance
                 */
                SelectedProbe.prototype.generation = null;

                /**
                 * SelectedProbe boundary.
                 * @member {doramcap.provenance.v1.Boundary} boundary
                 * @memberof doramcap.provenance.v1.SelectedProbe
                 * @instance
                 */
                SelectedProbe.prototype.boundary = 0;

                /**
                 * SelectedProbe timestampNs.
                 * @member {number|Long|null|undefined} timestampNs
                 * @memberof doramcap.provenance.v1.SelectedProbe
                 * @instance
                 */
                SelectedProbe.prototype.timestampNs = null;

                /**
                 * SelectedProbe clockDomain.
                 * @member {string} clockDomain
                 * @memberof doramcap.provenance.v1.SelectedProbe
                 * @instance
                 */
                SelectedProbe.prototype.clockDomain = "";

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(SelectedProbe.prototype, "_timestampNs", {
                    get: $util.oneOfGetter($oneOfFields = ["timestampNs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified SelectedProbe message. Does not implicitly {@link doramcap.provenance.v1.SelectedProbe.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance.v1.SelectedProbe
                 * @static
                 * @param {doramcap.provenance.v1.SelectedProbe.$Properties} message SelectedProbe message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                SelectedProbe.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.generation != null && $Object.hasOwnProperty.call(message, "generation"))
                        $root.doramcap.provenance.v1.GenerationRef.encode(message.generation, writer.uint32(/* id 1, wireType 2 =*/10).fork(), _depth + 1).ldelim();
                    if (message.boundary != null && $Object.hasOwnProperty.call(message, "boundary") && message.boundary !== 0)
                        writer.uint32(/* id 2, wireType 0 =*/16).int32(message.boundary);
                    if (message.timestampNs != null && $Object.hasOwnProperty.call(message, "timestampNs"))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.timestampNs);
                    if (message.clockDomain != null && $Object.hasOwnProperty.call(message, "clockDomain") && message.clockDomain !== "")
                        writer.uint32(/* id 4, wireType 2 =*/34).string(message.clockDomain);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a SelectedProbe message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance.v1.SelectedProbe
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.SelectedProbe & doramcap.provenance.v1.SelectedProbe.$Shape} SelectedProbe
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                SelectedProbe.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance.v1.SelectedProbe();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 2)
                                    break;
                                message.generation = $root.doramcap.provenance.v1.GenerationRef.decode(reader, reader.size(), $undefined, _depth + 1, message.generation);
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.int32())
                                    message.boundary = value;
                                else
                                    delete message.boundary;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                message.timestampNs = reader.uint64();
                                message._timestampNs = "timestampNs";
                                continue;
                            }
                        case 4: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.clockDomain = value;
                                else
                                    delete message.clockDomain;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a SelectedProbe message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance.v1.SelectedProbe
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance.v1.SelectedProbe} SelectedProbe
                 */
                SelectedProbe.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance.v1.SelectedProbe)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance.v1.SelectedProbe: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance.v1.SelectedProbe();
                    if (object.generation != null) {
                        if (!$util.isObject(object.generation))
                            throw $TypeError(".doramcap.provenance.v1.SelectedProbe.generation: object expected");
                        message.generation = $root.doramcap.provenance.v1.GenerationRef.fromObject(object.generation, _depth + 1);
                    }
                    if (object.boundary !== 0 && (typeof object.boundary !== "string" || $root.doramcap.provenance.v1.Boundary[object.boundary] !== 0))
                        switch (object.boundary) {
                        case "BOUNDARY_UNSPECIFIED":
                        case 0:
                            message.boundary = 0;
                            break;
                        case "START":
                        case 1:
                            message.boundary = 1;
                            break;
                        case "END":
                        case 2:
                            message.boundary = 2;
                            break;
                        default:
                            if (typeof object.boundary === "number" && (object.boundary | 0) === object.boundary)
                                message.boundary = object.boundary;
                        }
                    if (object.timestampNs != null)
                        if ($util.Long)
                            message.timestampNs = $util.Long.fromValue(object.timestampNs, true);
                        else if (typeof object.timestampNs === "string")
                            message.timestampNs = $parseInt(object.timestampNs, 10);
                        else if (typeof object.timestampNs === "number")
                            message.timestampNs = object.timestampNs;
                        else if (typeof object.timestampNs === "object")
                            message.timestampNs = new $util.LongBits(object.timestampNs.low >>> 0, object.timestampNs.high >>> 0).toNumber(true);
                    if (object.clockDomain != null)
                        if (typeof object.clockDomain !== "string" || object.clockDomain.length)
                            message.clockDomain = $String(object.clockDomain);
                    return message;
                };

                /**
                 * Creates a plain object from a SelectedProbe message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance.v1.SelectedProbe
                 * @static
                 * @param {doramcap.provenance.v1.SelectedProbe} message SelectedProbe
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                SelectedProbe.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.generation = null;
                        object.boundary = options.enums === $String ? "BOUNDARY_UNSPECIFIED" : 0;
                        object.clockDomain = "";
                    }
                    if (message.generation != null && $Object.hasOwnProperty.call(message, "generation"))
                        object.generation = $root.doramcap.provenance.v1.GenerationRef.toObject(message.generation, options, _depth + 1);
                    if (message.boundary != null && $Object.hasOwnProperty.call(message, "boundary"))
                        object.boundary = options.enums === $String ? $root.doramcap.provenance.v1.Boundary[message.boundary] === $undefined ? message.boundary : $root.doramcap.provenance.v1.Boundary[message.boundary] : message.boundary;
                    if (message.timestampNs != null && $Object.hasOwnProperty.call(message, "timestampNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.timestampNs = typeof message.timestampNs === "number" ? $BigInt(message.timestampNs) : $util.Long.fromBits(message.timestampNs.low >>> 0, message.timestampNs.high >>> 0, true).toBigInt();
                        else if (typeof message.timestampNs === "number")
                            object.timestampNs = options.longs === $String ? $String(message.timestampNs) : message.timestampNs;
                        else
                            object.timestampNs = options.longs === $String ? $util.Long.prototype.toString.call(message.timestampNs) : options.longs === $Number ? new $util.LongBits(message.timestampNs.low >>> 0, message.timestampNs.high >>> 0).toNumber(true) : message.timestampNs;
                    if (message.clockDomain != null && $Object.hasOwnProperty.call(message, "clockDomain"))
                        object.clockDomain = message.clockDomain;
                    return object;
                };

                /**
                 * Converts this SelectedProbe to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance.v1.SelectedProbe
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                SelectedProbe.prototype.toJSON = function() {
                    return SelectedProbe.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for SelectedProbe
                 * @function getTypeUrl
                 * @memberof doramcap.provenance.v1.SelectedProbe
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                SelectedProbe.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance.v1.SelectedProbe";
                };

                return SelectedProbe;
            })();

            v1.MeasurementResult = (function() {

                /**
                 * Properties of a MeasurementResult.
                 * @typedef {Object} doramcap.provenance.v1.MeasurementResult.$Properties
                 * @property {number|null} [version] MeasurementResult version
                 * @property {string|null} [measurement] MeasurementResult measurement
                 * @property {doramcap.provenance.v1.GenerationRef.$Properties|null} [anchor] MeasurementResult anchor
                 * @property {doramcap.provenance.v1.SelectedProbe.$Properties|null} [start] MeasurementResult start
                 * @property {doramcap.provenance.v1.SelectedProbe.$Properties|null} [end] MeasurementResult end
                 * @property {doramcap.provenance.v1.Status|null} [status] MeasurementResult status
                 * @property {doramcap.provenance.v1.Reason|null} [reason] MeasurementResult reason
                 * @property {number|Long|null} [durationNs] MeasurementResult durationNs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a MeasurementResult.
                 * @memberof doramcap.provenance.v1
                 * @interface IMeasurementResult
                 * @augments doramcap.provenance.v1.MeasurementResult.$Properties
                 * @deprecated Use doramcap.provenance.v1.MeasurementResult.$Properties instead.
                 */

                /**
                 * Shape of a MeasurementResult.
                 * @typedef {doramcap.provenance.v1.MeasurementResult.$Properties} doramcap.provenance.v1.MeasurementResult.$Shape
                 */

                /**
                 * Constructs a new MeasurementResult.
                 * @memberof doramcap.provenance.v1
                 * @classdesc Represents a MeasurementResult.
                 * @constructor
                 * @param {doramcap.provenance.v1.MeasurementResult.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const MeasurementResult = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * MeasurementResult version.
                 * @member {number} version
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @instance
                 */
                MeasurementResult.prototype.version = 0;

                /**
                 * MeasurementResult measurement.
                 * @member {string} measurement
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @instance
                 */
                MeasurementResult.prototype.measurement = "";

                /**
                 * MeasurementResult anchor.
                 * @member {doramcap.provenance.v1.GenerationRef.$Properties|null|undefined} anchor
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @instance
                 */
                MeasurementResult.prototype.anchor = null;

                /**
                 * MeasurementResult start.
                 * @member {doramcap.provenance.v1.SelectedProbe.$Properties|null|undefined} start
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @instance
                 */
                MeasurementResult.prototype.start = null;

                /**
                 * MeasurementResult end.
                 * @member {doramcap.provenance.v1.SelectedProbe.$Properties|null|undefined} end
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @instance
                 */
                MeasurementResult.prototype.end = null;

                /**
                 * MeasurementResult status.
                 * @member {doramcap.provenance.v1.Status} status
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @instance
                 */
                MeasurementResult.prototype.status = 0;

                /**
                 * MeasurementResult reason.
                 * @member {doramcap.provenance.v1.Reason} reason
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @instance
                 */
                MeasurementResult.prototype.reason = 0;

                /**
                 * MeasurementResult durationNs.
                 * @member {number|Long|null|undefined} durationNs
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @instance
                 */
                MeasurementResult.prototype.durationNs = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(MeasurementResult.prototype, "_durationNs", {
                    get: $util.oneOfGetter($oneOfFields = ["durationNs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified MeasurementResult message. Does not implicitly {@link doramcap.provenance.v1.MeasurementResult.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @static
                 * @param {doramcap.provenance.v1.MeasurementResult.$Properties} message MeasurementResult message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                MeasurementResult.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version") && message.version !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.version);
                    if (message.measurement != null && $Object.hasOwnProperty.call(message, "measurement") && message.measurement !== "")
                        writer.uint32(/* id 2, wireType 2 =*/18).string(message.measurement);
                    if (message.anchor != null && $Object.hasOwnProperty.call(message, "anchor"))
                        $root.doramcap.provenance.v1.GenerationRef.encode(message.anchor, writer.uint32(/* id 3, wireType 2 =*/26).fork(), _depth + 1).ldelim();
                    if (message.start != null && $Object.hasOwnProperty.call(message, "start"))
                        $root.doramcap.provenance.v1.SelectedProbe.encode(message.start, writer.uint32(/* id 4, wireType 2 =*/34).fork(), _depth + 1).ldelim();
                    if (message.end != null && $Object.hasOwnProperty.call(message, "end"))
                        $root.doramcap.provenance.v1.SelectedProbe.encode(message.end, writer.uint32(/* id 5, wireType 2 =*/42).fork(), _depth + 1).ldelim();
                    if (message.status != null && $Object.hasOwnProperty.call(message, "status") && message.status !== 0)
                        writer.uint32(/* id 6, wireType 0 =*/48).int32(message.status);
                    if (message.reason != null && $Object.hasOwnProperty.call(message, "reason") && message.reason !== 0)
                        writer.uint32(/* id 7, wireType 0 =*/56).int32(message.reason);
                    if (message.durationNs != null && $Object.hasOwnProperty.call(message, "durationNs"))
                        writer.uint32(/* id 8, wireType 0 =*/64).uint64(message.durationNs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a MeasurementResult message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.MeasurementResult & doramcap.provenance.v1.MeasurementResult.$Shape} MeasurementResult
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                MeasurementResult.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.provenance.v1.MeasurementResult();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.version = value;
                                else
                                    delete message.version;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.measurement = value;
                                else
                                    delete message.measurement;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                message.anchor = $root.doramcap.provenance.v1.GenerationRef.decode(reader, reader.size(), $undefined, _depth + 1, message.anchor);
                                continue;
                            }
                        case 4: {
                                if (wireType !== 2)
                                    break;
                                message.start = $root.doramcap.provenance.v1.SelectedProbe.decode(reader, reader.size(), $undefined, _depth + 1, message.start);
                                continue;
                            }
                        case 5: {
                                if (wireType !== 2)
                                    break;
                                message.end = $root.doramcap.provenance.v1.SelectedProbe.decode(reader, reader.size(), $undefined, _depth + 1, message.end);
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.int32())
                                    message.status = value;
                                else
                                    delete message.status;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.int32())
                                    message.reason = value;
                                else
                                    delete message.reason;
                                continue;
                            }
                        case 8: {
                                if (wireType !== 0)
                                    break;
                                message.durationNs = reader.uint64();
                                message._durationNs = "durationNs";
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a MeasurementResult message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.provenance.v1.MeasurementResult} MeasurementResult
                 */
                MeasurementResult.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.provenance.v1.MeasurementResult)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.provenance.v1.MeasurementResult: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.provenance.v1.MeasurementResult();
                    if (object.version != null)
                        if ($Number(object.version) !== 0)
                            message.version = object.version >>> 0;
                    if (object.measurement != null)
                        if (typeof object.measurement !== "string" || object.measurement.length)
                            message.measurement = $String(object.measurement);
                    if (object.anchor != null) {
                        if (!$util.isObject(object.anchor))
                            throw $TypeError(".doramcap.provenance.v1.MeasurementResult.anchor: object expected");
                        message.anchor = $root.doramcap.provenance.v1.GenerationRef.fromObject(object.anchor, _depth + 1);
                    }
                    if (object.start != null) {
                        if (!$util.isObject(object.start))
                            throw $TypeError(".doramcap.provenance.v1.MeasurementResult.start: object expected");
                        message.start = $root.doramcap.provenance.v1.SelectedProbe.fromObject(object.start, _depth + 1);
                    }
                    if (object.end != null) {
                        if (!$util.isObject(object.end))
                            throw $TypeError(".doramcap.provenance.v1.MeasurementResult.end: object expected");
                        message.end = $root.doramcap.provenance.v1.SelectedProbe.fromObject(object.end, _depth + 1);
                    }
                    if (object.status !== 0 && (typeof object.status !== "string" || $root.doramcap.provenance.v1.Status[object.status] !== 0))
                        switch (object.status) {
                        case "STATUS_UNSPECIFIED":
                        case 0:
                            message.status = 0;
                            break;
                        case "COMPLETE":
                        case 1:
                            message.status = 1;
                            break;
                        case "INCOMPLETE":
                        case 2:
                            message.status = 2;
                            break;
                        default:
                            if (typeof object.status === "number" && (object.status | 0) === object.status)
                                message.status = object.status;
                        }
                    if (object.reason !== 0 && (typeof object.reason !== "string" || $root.doramcap.provenance.v1.Reason[object.reason] !== 0))
                        switch (object.reason) {
                        case "REASON_UNSPECIFIED":
                        case 0:
                            message.reason = 0;
                            break;
                        case "HISTORY_EVICTED":
                        case 1:
                            message.reason = 1;
                            break;
                        case "ANCHOR_EVICTED":
                        case 2:
                            message.reason = 2;
                            break;
                        case "NO_MATCH":
                        case 3:
                            message.reason = 3;
                            break;
                        case "AMBIGUOUS":
                        case 4:
                            message.reason = 4;
                            break;
                        case "CLOCK_MISMATCH":
                        case 5:
                            message.reason = 5;
                            break;
                        case "INVALID_DURATION":
                        case 6:
                            message.reason = 6;
                            break;
                        case "TRAVERSAL_LIMIT":
                        case 7:
                            message.reason = 7;
                            break;
                        default:
                            if (typeof object.reason === "number" && (object.reason | 0) === object.reason)
                                message.reason = object.reason;
                        }
                    if (object.durationNs != null)
                        if ($util.Long)
                            message.durationNs = $util.Long.fromValue(object.durationNs, true);
                        else if (typeof object.durationNs === "string")
                            message.durationNs = $parseInt(object.durationNs, 10);
                        else if (typeof object.durationNs === "number")
                            message.durationNs = object.durationNs;
                        else if (typeof object.durationNs === "object")
                            message.durationNs = new $util.LongBits(object.durationNs.low >>> 0, object.durationNs.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a MeasurementResult message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @static
                 * @param {doramcap.provenance.v1.MeasurementResult} message MeasurementResult
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                MeasurementResult.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.version = 0;
                        object.measurement = "";
                        object.anchor = null;
                        object.start = null;
                        object.end = null;
                        object.status = options.enums === $String ? "STATUS_UNSPECIFIED" : 0;
                        object.reason = options.enums === $String ? "REASON_UNSPECIFIED" : 0;
                    }
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version"))
                        object.version = message.version;
                    if (message.measurement != null && $Object.hasOwnProperty.call(message, "measurement"))
                        object.measurement = message.measurement;
                    if (message.anchor != null && $Object.hasOwnProperty.call(message, "anchor"))
                        object.anchor = $root.doramcap.provenance.v1.GenerationRef.toObject(message.anchor, options, _depth + 1);
                    if (message.start != null && $Object.hasOwnProperty.call(message, "start"))
                        object.start = $root.doramcap.provenance.v1.SelectedProbe.toObject(message.start, options, _depth + 1);
                    if (message.end != null && $Object.hasOwnProperty.call(message, "end"))
                        object.end = $root.doramcap.provenance.v1.SelectedProbe.toObject(message.end, options, _depth + 1);
                    if (message.status != null && $Object.hasOwnProperty.call(message, "status"))
                        object.status = options.enums === $String ? $root.doramcap.provenance.v1.Status[message.status] === $undefined ? message.status : $root.doramcap.provenance.v1.Status[message.status] : message.status;
                    if (message.reason != null && $Object.hasOwnProperty.call(message, "reason"))
                        object.reason = options.enums === $String ? $root.doramcap.provenance.v1.Reason[message.reason] === $undefined ? message.reason : $root.doramcap.provenance.v1.Reason[message.reason] : message.reason;
                    if (message.durationNs != null && $Object.hasOwnProperty.call(message, "durationNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.durationNs = typeof message.durationNs === "number" ? $BigInt(message.durationNs) : $util.Long.fromBits(message.durationNs.low >>> 0, message.durationNs.high >>> 0, true).toBigInt();
                        else if (typeof message.durationNs === "number")
                            object.durationNs = options.longs === $String ? $String(message.durationNs) : message.durationNs;
                        else
                            object.durationNs = options.longs === $String ? $util.Long.prototype.toString.call(message.durationNs) : options.longs === $Number ? new $util.LongBits(message.durationNs.low >>> 0, message.durationNs.high >>> 0).toNumber(true) : message.durationNs;
                    return object;
                };

                /**
                 * Converts this MeasurementResult to JSON.
                 * @function toJSON
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                MeasurementResult.prototype.toJSON = function() {
                    return MeasurementResult.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for MeasurementResult
                 * @function getTypeUrl
                 * @memberof doramcap.provenance.v1.MeasurementResult
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                MeasurementResult.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.provenance.v1.MeasurementResult";
                };

                return MeasurementResult;
            })();

            return v1;
        })();

        return provenance;
    })();

    doramcap.activity = (function() {

        /**
         * Namespace activity.
         * @memberof doramcap
         * @namespace
         */
        const activity = {};

        activity.v1 = (function() {

            /**
             * Namespace v1.
             * @memberof doramcap.activity
             * @namespace
             */
            const v1 = {};

            v1.Batch = (function() {

                /**
                 * Properties of a Batch.
                 * @typedef {Object} doramcap.activity.v1.Batch.$Properties
                 * @property {number|null} [version] Batch version
                 * @property {number|Long|null} [session] Batch session
                 * @property {number|Long|null} [sequence] Batch sequence
                 * @property {number|Long|null} [unixOriginNs] Batch unixOriginNs
                 * @property {number|Long|null} [droppedRecords] Batch droppedRecords
                 * @property {number|null} [pid] Batch pid
                 * @property {Array.<string>|null} [dictionary] Batch dictionary
                 * @property {Uint8Array|null} [strings] Batch strings
                 * @property {Array.<doramcap.activity.v1.Record.$Properties>|null} [records] Batch records
                 * @property {number|Long|null} [droppedFields] Batch droppedFields
                 * @property {number|Long|null} [rejectedSpans] Batch rejectedSpans
                 * @property {number|Long|null} [unregisteredRecords] Batch unregisteredRecords
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Batch.
                 * @memberof doramcap.activity.v1
                 * @interface IBatch
                 * @augments doramcap.activity.v1.Batch.$Properties
                 * @deprecated Use doramcap.activity.v1.Batch.$Properties instead.
                 */

                /**
                 * Shape of a Batch.
                 * @typedef {doramcap.activity.v1.Batch.$Properties} doramcap.activity.v1.Batch.$Shape
                 */

                /**
                 * Constructs a new Batch.
                 * @memberof doramcap.activity.v1
                 * @classdesc Represents a Batch.
                 * @constructor
                 * @param {doramcap.activity.v1.Batch.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Batch = function (properties) {
                    this.dictionary = [];
                    this.records = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Batch version.
                 * @member {number} version
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.version = 0;

                /**
                 * Batch session.
                 * @member {number|Long} session
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.session = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Batch sequence.
                 * @member {number|Long} sequence
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.sequence = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Batch unixOriginNs.
                 * @member {number|Long} unixOriginNs
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.unixOriginNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Batch droppedRecords.
                 * @member {number|Long} droppedRecords
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.droppedRecords = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Batch pid.
                 * @member {number} pid
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.pid = 0;

                /**
                 * Batch dictionary.
                 * @member {Array.<string>} dictionary
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.dictionary = $util.emptyArray;

                /**
                 * Batch strings.
                 * @member {Uint8Array} strings
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.strings = $util.newBuffer([]);

                /**
                 * Batch records.
                 * @member {Array.<doramcap.activity.v1.Record.$Properties>} records
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.records = $util.emptyArray;

                /**
                 * Batch droppedFields.
                 * @member {number|Long} droppedFields
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.droppedFields = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Batch rejectedSpans.
                 * @member {number|Long} rejectedSpans
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.rejectedSpans = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Batch unregisteredRecords.
                 * @member {number|Long} unregisteredRecords
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 */
                Batch.prototype.unregisteredRecords = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified Batch message. Does not implicitly {@link doramcap.activity.v1.Batch.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.activity.v1.Batch
                 * @static
                 * @param {doramcap.activity.v1.Batch.$Properties} message Batch message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Batch.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version") && message.version !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.version);
                    if (message.session != null && $Object.hasOwnProperty.call(message, "session") && (typeof message.session === "object" ? message.session.low || message.session.high : message.session !== 0))
                        writer.uint32(/* id 2, wireType 1 =*/17).fixed64(message.session);
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence") && (typeof message.sequence === "object" ? message.sequence.low || message.sequence.high : message.sequence !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.sequence);
                    if (message.unixOriginNs != null && $Object.hasOwnProperty.call(message, "unixOriginNs") && (typeof message.unixOriginNs === "object" ? message.unixOriginNs.low || message.unixOriginNs.high : message.unixOriginNs !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.unixOriginNs);
                    if (message.droppedRecords != null && $Object.hasOwnProperty.call(message, "droppedRecords") && (typeof message.droppedRecords === "object" ? message.droppedRecords.low || message.droppedRecords.high : message.droppedRecords !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.droppedRecords);
                    if (message.pid != null && $Object.hasOwnProperty.call(message, "pid") && message.pid !== 0)
                        writer.uint32(/* id 6, wireType 0 =*/48).uint32(message.pid);
                    if (message.dictionary != null && message.dictionary.length)
                        for (let i = 0; i < message.dictionary.length; ++i)
                            writer.uint32(/* id 7, wireType 2 =*/58).string(message.dictionary[i]);
                    if (message.strings != null && $Object.hasOwnProperty.call(message, "strings") && message.strings.length)
                        writer.uint32(/* id 8, wireType 2 =*/66).bytes(message.strings);
                    if (message.records != null && message.records.length)
                        for (let i = 0; i < message.records.length; ++i)
                            $root.doramcap.activity.v1.Record.encode(message.records[i], writer.uint32(/* id 9, wireType 2 =*/74).fork(), _depth + 1).ldelim();
                    if (message.droppedFields != null && $Object.hasOwnProperty.call(message, "droppedFields") && (typeof message.droppedFields === "object" ? message.droppedFields.low || message.droppedFields.high : message.droppedFields !== 0))
                        writer.uint32(/* id 10, wireType 0 =*/80).uint64(message.droppedFields);
                    if (message.rejectedSpans != null && $Object.hasOwnProperty.call(message, "rejectedSpans") && (typeof message.rejectedSpans === "object" ? message.rejectedSpans.low || message.rejectedSpans.high : message.rejectedSpans !== 0))
                        writer.uint32(/* id 11, wireType 0 =*/88).uint64(message.rejectedSpans);
                    if (message.unregisteredRecords != null && $Object.hasOwnProperty.call(message, "unregisteredRecords") && (typeof message.unregisteredRecords === "object" ? message.unregisteredRecords.low || message.unregisteredRecords.high : message.unregisteredRecords !== 0))
                        writer.uint32(/* id 12, wireType 0 =*/96).uint64(message.unregisteredRecords);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Batch message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.activity.v1.Batch
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.activity.v1.Batch & doramcap.activity.v1.Batch.$Shape} Batch
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Batch.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.activity.v1.Batch();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.version = value;
                                else
                                    delete message.version;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 1)
                                    break;
                                if (typeof (value = reader.fixed64()) === "object" ? value.low || value.high : value !== 0)
                                    message.session = value;
                                else
                                    delete message.session;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.sequence = value;
                                else
                                    delete message.sequence;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.unixOriginNs = value;
                                else
                                    delete message.unixOriginNs;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.droppedRecords = value;
                                else
                                    delete message.droppedRecords;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.pid = value;
                                else
                                    delete message.pid;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.dictionary && message.dictionary.length))
                                    message.dictionary = [];
                                message.dictionary.push(reader.stringVerify());
                                continue;
                            }
                        case 8: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.bytes()).length)
                                    message.strings = value;
                                else
                                    delete message.strings;
                                continue;
                            }
                        case 9: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.records && message.records.length))
                                    message.records = [];
                                message.records.push($root.doramcap.activity.v1.Record.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 10: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.droppedFields = value;
                                else
                                    delete message.droppedFields;
                                continue;
                            }
                        case 11: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.rejectedSpans = value;
                                else
                                    delete message.rejectedSpans;
                                continue;
                            }
                        case 12: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.unregisteredRecords = value;
                                else
                                    delete message.unregisteredRecords;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a Batch message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.activity.v1.Batch
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.activity.v1.Batch} Batch
                 */
                Batch.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.activity.v1.Batch)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.activity.v1.Batch: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.activity.v1.Batch();
                    if (object.version != null)
                        if ($Number(object.version) !== 0)
                            message.version = object.version >>> 0;
                    if (object.session != null)
                        if (typeof object.session === "object" ? object.session.low || object.session.high : $Number(object.session) !== 0)
                            if ($util.Long)
                                message.session = $util.Long.fromValue(object.session, true);
                            else if (typeof object.session === "string")
                                message.session = $parseInt(object.session, 10);
                            else if (typeof object.session === "number")
                                message.session = object.session;
                            else if (typeof object.session === "object")
                                message.session = new $util.LongBits(object.session.low >>> 0, object.session.high >>> 0).toNumber(true);
                    if (object.sequence != null)
                        if (typeof object.sequence === "object" ? object.sequence.low || object.sequence.high : $Number(object.sequence) !== 0)
                            if ($util.Long)
                                message.sequence = $util.Long.fromValue(object.sequence, true);
                            else if (typeof object.sequence === "string")
                                message.sequence = $parseInt(object.sequence, 10);
                            else if (typeof object.sequence === "number")
                                message.sequence = object.sequence;
                            else if (typeof object.sequence === "object")
                                message.sequence = new $util.LongBits(object.sequence.low >>> 0, object.sequence.high >>> 0).toNumber(true);
                    if (object.unixOriginNs != null)
                        if (typeof object.unixOriginNs === "object" ? object.unixOriginNs.low || object.unixOriginNs.high : $Number(object.unixOriginNs) !== 0)
                            if ($util.Long)
                                message.unixOriginNs = $util.Long.fromValue(object.unixOriginNs, true);
                            else if (typeof object.unixOriginNs === "string")
                                message.unixOriginNs = $parseInt(object.unixOriginNs, 10);
                            else if (typeof object.unixOriginNs === "number")
                                message.unixOriginNs = object.unixOriginNs;
                            else if (typeof object.unixOriginNs === "object")
                                message.unixOriginNs = new $util.LongBits(object.unixOriginNs.low >>> 0, object.unixOriginNs.high >>> 0).toNumber(true);
                    if (object.droppedRecords != null)
                        if (typeof object.droppedRecords === "object" ? object.droppedRecords.low || object.droppedRecords.high : $Number(object.droppedRecords) !== 0)
                            if ($util.Long)
                                message.droppedRecords = $util.Long.fromValue(object.droppedRecords, true);
                            else if (typeof object.droppedRecords === "string")
                                message.droppedRecords = $parseInt(object.droppedRecords, 10);
                            else if (typeof object.droppedRecords === "number")
                                message.droppedRecords = object.droppedRecords;
                            else if (typeof object.droppedRecords === "object")
                                message.droppedRecords = new $util.LongBits(object.droppedRecords.low >>> 0, object.droppedRecords.high >>> 0).toNumber(true);
                    if (object.pid != null)
                        if ($Number(object.pid) !== 0)
                            message.pid = object.pid >>> 0;
                    if (object.dictionary) {
                        if (!$Array.isArray(object.dictionary))
                            throw $TypeError(".doramcap.activity.v1.Batch.dictionary: array expected");
                        message.dictionary = $Array(object.dictionary.length);
                        for (let i = 0; i < object.dictionary.length; ++i)
                            message.dictionary[i] = $String(object.dictionary[i]);
                    }
                    if (object.strings != null)
                        if (object.strings.length)
                            if (typeof object.strings === "string")
                                $util.base64.decode(object.strings, message.strings = $util.newBuffer($util.base64.length(object.strings)), 0);
                            else if (object.strings.length >= 0)
                                message.strings = object.strings;
                    if (object.records) {
                        if (!$Array.isArray(object.records))
                            throw $TypeError(".doramcap.activity.v1.Batch.records: array expected");
                        message.records = $Array(object.records.length);
                        for (let i = 0; i < object.records.length; ++i) {
                            if (!$util.isObject(object.records[i]))
                                throw $TypeError(".doramcap.activity.v1.Batch.records: object expected");
                            message.records[i] = $root.doramcap.activity.v1.Record.fromObject(object.records[i], _depth + 1);
                        }
                    }
                    if (object.droppedFields != null)
                        if (typeof object.droppedFields === "object" ? object.droppedFields.low || object.droppedFields.high : $Number(object.droppedFields) !== 0)
                            if ($util.Long)
                                message.droppedFields = $util.Long.fromValue(object.droppedFields, true);
                            else if (typeof object.droppedFields === "string")
                                message.droppedFields = $parseInt(object.droppedFields, 10);
                            else if (typeof object.droppedFields === "number")
                                message.droppedFields = object.droppedFields;
                            else if (typeof object.droppedFields === "object")
                                message.droppedFields = new $util.LongBits(object.droppedFields.low >>> 0, object.droppedFields.high >>> 0).toNumber(true);
                    if (object.rejectedSpans != null)
                        if (typeof object.rejectedSpans === "object" ? object.rejectedSpans.low || object.rejectedSpans.high : $Number(object.rejectedSpans) !== 0)
                            if ($util.Long)
                                message.rejectedSpans = $util.Long.fromValue(object.rejectedSpans, true);
                            else if (typeof object.rejectedSpans === "string")
                                message.rejectedSpans = $parseInt(object.rejectedSpans, 10);
                            else if (typeof object.rejectedSpans === "number")
                                message.rejectedSpans = object.rejectedSpans;
                            else if (typeof object.rejectedSpans === "object")
                                message.rejectedSpans = new $util.LongBits(object.rejectedSpans.low >>> 0, object.rejectedSpans.high >>> 0).toNumber(true);
                    if (object.unregisteredRecords != null)
                        if (typeof object.unregisteredRecords === "object" ? object.unregisteredRecords.low || object.unregisteredRecords.high : $Number(object.unregisteredRecords) !== 0)
                            if ($util.Long)
                                message.unregisteredRecords = $util.Long.fromValue(object.unregisteredRecords, true);
                            else if (typeof object.unregisteredRecords === "string")
                                message.unregisteredRecords = $parseInt(object.unregisteredRecords, 10);
                            else if (typeof object.unregisteredRecords === "number")
                                message.unregisteredRecords = object.unregisteredRecords;
                            else if (typeof object.unregisteredRecords === "object")
                                message.unregisteredRecords = new $util.LongBits(object.unregisteredRecords.low >>> 0, object.unregisteredRecords.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a Batch message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.activity.v1.Batch
                 * @static
                 * @param {doramcap.activity.v1.Batch} message Batch
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Batch.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.arrays || options.defaults) {
                        object.dictionary = [];
                        object.records = [];
                    }
                    if (options.defaults) {
                        object.version = 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.session = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.session = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.sequence = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.sequence = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.unixOriginNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.unixOriginNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.droppedRecords = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.droppedRecords = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.pid = 0;
                        if (options.bytes === $String)
                            object.strings = "";
                        else {
                            object.strings = [];
                            if (options.bytes !== $Array)
                                object.strings = $util.newBuffer(object.strings);
                        }
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.droppedFields = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.droppedFields = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.rejectedSpans = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.rejectedSpans = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.unregisteredRecords = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.unregisteredRecords = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version"))
                        object.version = message.version;
                    if (message.session != null && $Object.hasOwnProperty.call(message, "session"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.session = typeof message.session === "number" ? $BigInt(message.session) : $util.Long.fromBits(message.session.low >>> 0, message.session.high >>> 0, true).toBigInt();
                        else if (typeof message.session === "number")
                            object.session = options.longs === $String ? $String(message.session) : message.session;
                        else
                            object.session = options.longs === $String ? $util.Long.prototype.toString.call(message.session) : options.longs === $Number ? new $util.LongBits(message.session.low >>> 0, message.session.high >>> 0).toNumber(true) : message.session;
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.sequence = typeof message.sequence === "number" ? $BigInt(message.sequence) : $util.Long.fromBits(message.sequence.low >>> 0, message.sequence.high >>> 0, true).toBigInt();
                        else if (typeof message.sequence === "number")
                            object.sequence = options.longs === $String ? $String(message.sequence) : message.sequence;
                        else
                            object.sequence = options.longs === $String ? $util.Long.prototype.toString.call(message.sequence) : options.longs === $Number ? new $util.LongBits(message.sequence.low >>> 0, message.sequence.high >>> 0).toNumber(true) : message.sequence;
                    if (message.unixOriginNs != null && $Object.hasOwnProperty.call(message, "unixOriginNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.unixOriginNs = typeof message.unixOriginNs === "number" ? $BigInt(message.unixOriginNs) : $util.Long.fromBits(message.unixOriginNs.low >>> 0, message.unixOriginNs.high >>> 0, true).toBigInt();
                        else if (typeof message.unixOriginNs === "number")
                            object.unixOriginNs = options.longs === $String ? $String(message.unixOriginNs) : message.unixOriginNs;
                        else
                            object.unixOriginNs = options.longs === $String ? $util.Long.prototype.toString.call(message.unixOriginNs) : options.longs === $Number ? new $util.LongBits(message.unixOriginNs.low >>> 0, message.unixOriginNs.high >>> 0).toNumber(true) : message.unixOriginNs;
                    if (message.droppedRecords != null && $Object.hasOwnProperty.call(message, "droppedRecords"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.droppedRecords = typeof message.droppedRecords === "number" ? $BigInt(message.droppedRecords) : $util.Long.fromBits(message.droppedRecords.low >>> 0, message.droppedRecords.high >>> 0, true).toBigInt();
                        else if (typeof message.droppedRecords === "number")
                            object.droppedRecords = options.longs === $String ? $String(message.droppedRecords) : message.droppedRecords;
                        else
                            object.droppedRecords = options.longs === $String ? $util.Long.prototype.toString.call(message.droppedRecords) : options.longs === $Number ? new $util.LongBits(message.droppedRecords.low >>> 0, message.droppedRecords.high >>> 0).toNumber(true) : message.droppedRecords;
                    if (message.pid != null && $Object.hasOwnProperty.call(message, "pid"))
                        object.pid = message.pid;
                    if (message.dictionary && message.dictionary.length) {
                        object.dictionary = $Array(message.dictionary.length);
                        for (let j = 0; j < message.dictionary.length; ++j)
                            object.dictionary[j] = message.dictionary[j];
                    }
                    if (message.strings != null && $Object.hasOwnProperty.call(message, "strings"))
                        object.strings = options.bytes === $String ? $util.base64.encode(message.strings, 0, message.strings.length) : options.bytes === $Array ? $Array.prototype.slice.call(message.strings) : message.strings;
                    if (message.records && message.records.length) {
                        object.records = $Array(message.records.length);
                        for (let j = 0; j < message.records.length; ++j)
                            object.records[j] = $root.doramcap.activity.v1.Record.toObject(message.records[j], options, _depth + 1);
                    }
                    if (message.droppedFields != null && $Object.hasOwnProperty.call(message, "droppedFields"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.droppedFields = typeof message.droppedFields === "number" ? $BigInt(message.droppedFields) : $util.Long.fromBits(message.droppedFields.low >>> 0, message.droppedFields.high >>> 0, true).toBigInt();
                        else if (typeof message.droppedFields === "number")
                            object.droppedFields = options.longs === $String ? $String(message.droppedFields) : message.droppedFields;
                        else
                            object.droppedFields = options.longs === $String ? $util.Long.prototype.toString.call(message.droppedFields) : options.longs === $Number ? new $util.LongBits(message.droppedFields.low >>> 0, message.droppedFields.high >>> 0).toNumber(true) : message.droppedFields;
                    if (message.rejectedSpans != null && $Object.hasOwnProperty.call(message, "rejectedSpans"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.rejectedSpans = typeof message.rejectedSpans === "number" ? $BigInt(message.rejectedSpans) : $util.Long.fromBits(message.rejectedSpans.low >>> 0, message.rejectedSpans.high >>> 0, true).toBigInt();
                        else if (typeof message.rejectedSpans === "number")
                            object.rejectedSpans = options.longs === $String ? $String(message.rejectedSpans) : message.rejectedSpans;
                        else
                            object.rejectedSpans = options.longs === $String ? $util.Long.prototype.toString.call(message.rejectedSpans) : options.longs === $Number ? new $util.LongBits(message.rejectedSpans.low >>> 0, message.rejectedSpans.high >>> 0).toNumber(true) : message.rejectedSpans;
                    if (message.unregisteredRecords != null && $Object.hasOwnProperty.call(message, "unregisteredRecords"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.unregisteredRecords = typeof message.unregisteredRecords === "number" ? $BigInt(message.unregisteredRecords) : $util.Long.fromBits(message.unregisteredRecords.low >>> 0, message.unregisteredRecords.high >>> 0, true).toBigInt();
                        else if (typeof message.unregisteredRecords === "number")
                            object.unregisteredRecords = options.longs === $String ? $String(message.unregisteredRecords) : message.unregisteredRecords;
                        else
                            object.unregisteredRecords = options.longs === $String ? $util.Long.prototype.toString.call(message.unregisteredRecords) : options.longs === $Number ? new $util.LongBits(message.unregisteredRecords.low >>> 0, message.unregisteredRecords.high >>> 0).toNumber(true) : message.unregisteredRecords;
                    return object;
                };

                /**
                 * Converts this Batch to JSON.
                 * @function toJSON
                 * @memberof doramcap.activity.v1.Batch
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Batch.prototype.toJSON = function() {
                    return Batch.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Batch
                 * @function getTypeUrl
                 * @memberof doramcap.activity.v1.Batch
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Batch.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.activity.v1.Batch";
                };

                return Batch;
            })();

            v1.Record = (function() {

                /**
                 * Properties of a Record.
                 * @typedef {Object} doramcap.activity.v1.Record.$Properties
                 * @property {doramcap.activity.v1.Record.Kind|null} [kind] Record kind
                 * @property {number|Long|null} [monotonicNs] Record monotonicNs
                 * @property {number|Long|null} [spanId] Record spanId
                 * @property {number|Long|null} [traceRoot] Record traceRoot
                 * @property {number|Long|null} [parentId] Record parentId
                 * @property {number|Long|null} [linkedId] Record linkedId
                 * @property {number|Long|null} [linkedRoot] Record linkedRoot
                 * @property {number|null} [tid] Record tid
                 * @property {number|null} [threadName] Record threadName
                 * @property {number|null} [name] Record name
                 * @property {Array.<doramcap.activity.v1.Field.$Properties>|null} [fields] Record fields
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Record.
                 * @memberof doramcap.activity.v1
                 * @interface IRecord
                 * @augments doramcap.activity.v1.Record.$Properties
                 * @deprecated Use doramcap.activity.v1.Record.$Properties instead.
                 */

                /**
                 * Shape of a Record.
                 * @typedef {doramcap.activity.v1.Record.$Properties} doramcap.activity.v1.Record.$Shape
                 */

                /**
                 * Constructs a new Record.
                 * @memberof doramcap.activity.v1
                 * @classdesc Represents a Record.
                 * @constructor
                 * @param {doramcap.activity.v1.Record.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Record = function (properties) {
                    this.fields = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Record kind.
                 * @member {doramcap.activity.v1.Record.Kind} kind
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.kind = 0;

                /**
                 * Record monotonicNs.
                 * @member {number|Long} monotonicNs
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.monotonicNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Record spanId.
                 * @member {number|Long} spanId
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.spanId = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Record traceRoot.
                 * @member {number|Long} traceRoot
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.traceRoot = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Record parentId.
                 * @member {number|Long} parentId
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.parentId = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Record linkedId.
                 * @member {number|Long} linkedId
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.linkedId = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Record linkedRoot.
                 * @member {number|Long} linkedRoot
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.linkedRoot = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Record tid.
                 * @member {number} tid
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.tid = 0;

                /**
                 * Record threadName.
                 * @member {number} threadName
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.threadName = 0;

                /**
                 * Record name.
                 * @member {number} name
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.name = 0;

                /**
                 * Record fields.
                 * @member {Array.<doramcap.activity.v1.Field.$Properties>} fields
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 */
                Record.prototype.fields = $util.emptyArray;

                /**
                 * Encodes the specified Record message. Does not implicitly {@link doramcap.activity.v1.Record.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.activity.v1.Record
                 * @static
                 * @param {doramcap.activity.v1.Record.$Properties} message Record message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Record.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind") && message.kind !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).int32(message.kind);
                    if (message.monotonicNs != null && $Object.hasOwnProperty.call(message, "monotonicNs") && (typeof message.monotonicNs === "object" ? message.monotonicNs.low || message.monotonicNs.high : message.monotonicNs !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.monotonicNs);
                    if (message.spanId != null && $Object.hasOwnProperty.call(message, "spanId") && (typeof message.spanId === "object" ? message.spanId.low || message.spanId.high : message.spanId !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.spanId);
                    if (message.traceRoot != null && $Object.hasOwnProperty.call(message, "traceRoot") && (typeof message.traceRoot === "object" ? message.traceRoot.low || message.traceRoot.high : message.traceRoot !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.traceRoot);
                    if (message.parentId != null && $Object.hasOwnProperty.call(message, "parentId") && (typeof message.parentId === "object" ? message.parentId.low || message.parentId.high : message.parentId !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.parentId);
                    if (message.linkedId != null && $Object.hasOwnProperty.call(message, "linkedId") && (typeof message.linkedId === "object" ? message.linkedId.low || message.linkedId.high : message.linkedId !== 0))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.linkedId);
                    if (message.linkedRoot != null && $Object.hasOwnProperty.call(message, "linkedRoot") && (typeof message.linkedRoot === "object" ? message.linkedRoot.low || message.linkedRoot.high : message.linkedRoot !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.linkedRoot);
                    if (message.tid != null && $Object.hasOwnProperty.call(message, "tid") && message.tid !== 0)
                        writer.uint32(/* id 8, wireType 0 =*/64).uint32(message.tid);
                    if (message.threadName != null && $Object.hasOwnProperty.call(message, "threadName") && message.threadName !== 0)
                        writer.uint32(/* id 9, wireType 0 =*/72).uint32(message.threadName);
                    if (message.name != null && $Object.hasOwnProperty.call(message, "name") && message.name !== 0)
                        writer.uint32(/* id 10, wireType 0 =*/80).uint32(message.name);
                    if (message.fields != null && message.fields.length)
                        for (let i = 0; i < message.fields.length; ++i)
                            $root.doramcap.activity.v1.Field.encode(message.fields[i], writer.uint32(/* id 11, wireType 2 =*/90).fork(), _depth + 1).ldelim();
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Record message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.activity.v1.Record
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.activity.v1.Record & doramcap.activity.v1.Record.$Shape} Record
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Record.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.activity.v1.Record();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.int32())
                                    message.kind = value;
                                else
                                    delete message.kind;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.monotonicNs = value;
                                else
                                    delete message.monotonicNs;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.spanId = value;
                                else
                                    delete message.spanId;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.traceRoot = value;
                                else
                                    delete message.traceRoot;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.parentId = value;
                                else
                                    delete message.parentId;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.linkedId = value;
                                else
                                    delete message.linkedId;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.linkedRoot = value;
                                else
                                    delete message.linkedRoot;
                                continue;
                            }
                        case 8: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.tid = value;
                                else
                                    delete message.tid;
                                continue;
                            }
                        case 9: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.threadName = value;
                                else
                                    delete message.threadName;
                                continue;
                            }
                        case 10: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.name = value;
                                else
                                    delete message.name;
                                continue;
                            }
                        case 11: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.fields && message.fields.length))
                                    message.fields = [];
                                message.fields.push($root.doramcap.activity.v1.Field.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a Record message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.activity.v1.Record
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.activity.v1.Record} Record
                 */
                Record.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.activity.v1.Record)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.activity.v1.Record: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.activity.v1.Record();
                    if (object.kind !== 0 && (typeof object.kind !== "string" || $root.doramcap.activity.v1.Record.Kind[object.kind] !== 0))
                        switch (object.kind) {
                        case "BEGIN":
                        case 0:
                            message.kind = 0;
                            break;
                        case "EVENT":
                        case 1:
                            message.kind = 1;
                            break;
                        case "UPDATE":
                        case 2:
                            message.kind = 2;
                            break;
                        case "LINK":
                        case 3:
                            message.kind = 3;
                            break;
                        case "END":
                        case 4:
                            message.kind = 4;
                            break;
                        default:
                            if (typeof object.kind === "number" && (object.kind | 0) === object.kind)
                                message.kind = object.kind;
                        }
                    if (object.monotonicNs != null)
                        if (typeof object.monotonicNs === "object" ? object.monotonicNs.low || object.monotonicNs.high : $Number(object.monotonicNs) !== 0)
                            if ($util.Long)
                                message.monotonicNs = $util.Long.fromValue(object.monotonicNs, true);
                            else if (typeof object.monotonicNs === "string")
                                message.monotonicNs = $parseInt(object.monotonicNs, 10);
                            else if (typeof object.monotonicNs === "number")
                                message.monotonicNs = object.monotonicNs;
                            else if (typeof object.monotonicNs === "object")
                                message.monotonicNs = new $util.LongBits(object.monotonicNs.low >>> 0, object.monotonicNs.high >>> 0).toNumber(true);
                    if (object.spanId != null)
                        if (typeof object.spanId === "object" ? object.spanId.low || object.spanId.high : $Number(object.spanId) !== 0)
                            if ($util.Long)
                                message.spanId = $util.Long.fromValue(object.spanId, true);
                            else if (typeof object.spanId === "string")
                                message.spanId = $parseInt(object.spanId, 10);
                            else if (typeof object.spanId === "number")
                                message.spanId = object.spanId;
                            else if (typeof object.spanId === "object")
                                message.spanId = new $util.LongBits(object.spanId.low >>> 0, object.spanId.high >>> 0).toNumber(true);
                    if (object.traceRoot != null)
                        if (typeof object.traceRoot === "object" ? object.traceRoot.low || object.traceRoot.high : $Number(object.traceRoot) !== 0)
                            if ($util.Long)
                                message.traceRoot = $util.Long.fromValue(object.traceRoot, true);
                            else if (typeof object.traceRoot === "string")
                                message.traceRoot = $parseInt(object.traceRoot, 10);
                            else if (typeof object.traceRoot === "number")
                                message.traceRoot = object.traceRoot;
                            else if (typeof object.traceRoot === "object")
                                message.traceRoot = new $util.LongBits(object.traceRoot.low >>> 0, object.traceRoot.high >>> 0).toNumber(true);
                    if (object.parentId != null)
                        if (typeof object.parentId === "object" ? object.parentId.low || object.parentId.high : $Number(object.parentId) !== 0)
                            if ($util.Long)
                                message.parentId = $util.Long.fromValue(object.parentId, true);
                            else if (typeof object.parentId === "string")
                                message.parentId = $parseInt(object.parentId, 10);
                            else if (typeof object.parentId === "number")
                                message.parentId = object.parentId;
                            else if (typeof object.parentId === "object")
                                message.parentId = new $util.LongBits(object.parentId.low >>> 0, object.parentId.high >>> 0).toNumber(true);
                    if (object.linkedId != null)
                        if (typeof object.linkedId === "object" ? object.linkedId.low || object.linkedId.high : $Number(object.linkedId) !== 0)
                            if ($util.Long)
                                message.linkedId = $util.Long.fromValue(object.linkedId, true);
                            else if (typeof object.linkedId === "string")
                                message.linkedId = $parseInt(object.linkedId, 10);
                            else if (typeof object.linkedId === "number")
                                message.linkedId = object.linkedId;
                            else if (typeof object.linkedId === "object")
                                message.linkedId = new $util.LongBits(object.linkedId.low >>> 0, object.linkedId.high >>> 0).toNumber(true);
                    if (object.linkedRoot != null)
                        if (typeof object.linkedRoot === "object" ? object.linkedRoot.low || object.linkedRoot.high : $Number(object.linkedRoot) !== 0)
                            if ($util.Long)
                                message.linkedRoot = $util.Long.fromValue(object.linkedRoot, true);
                            else if (typeof object.linkedRoot === "string")
                                message.linkedRoot = $parseInt(object.linkedRoot, 10);
                            else if (typeof object.linkedRoot === "number")
                                message.linkedRoot = object.linkedRoot;
                            else if (typeof object.linkedRoot === "object")
                                message.linkedRoot = new $util.LongBits(object.linkedRoot.low >>> 0, object.linkedRoot.high >>> 0).toNumber(true);
                    if (object.tid != null)
                        if ($Number(object.tid) !== 0)
                            message.tid = object.tid >>> 0;
                    if (object.threadName != null)
                        if ($Number(object.threadName) !== 0)
                            message.threadName = object.threadName >>> 0;
                    if (object.name != null)
                        if ($Number(object.name) !== 0)
                            message.name = object.name >>> 0;
                    if (object.fields) {
                        if (!$Array.isArray(object.fields))
                            throw $TypeError(".doramcap.activity.v1.Record.fields: array expected");
                        message.fields = $Array(object.fields.length);
                        for (let i = 0; i < object.fields.length; ++i) {
                            if (!$util.isObject(object.fields[i]))
                                throw $TypeError(".doramcap.activity.v1.Record.fields: object expected");
                            message.fields[i] = $root.doramcap.activity.v1.Field.fromObject(object.fields[i], _depth + 1);
                        }
                    }
                    return message;
                };

                /**
                 * Creates a plain object from a Record message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.activity.v1.Record
                 * @static
                 * @param {doramcap.activity.v1.Record} message Record
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Record.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.arrays || options.defaults)
                        object.fields = [];
                    if (options.defaults) {
                        object.kind = options.enums === $String ? "BEGIN" : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.monotonicNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.monotonicNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.spanId = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.spanId = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.traceRoot = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.traceRoot = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.parentId = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.parentId = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.linkedId = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.linkedId = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.linkedRoot = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.linkedRoot = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.tid = 0;
                        object.threadName = 0;
                        object.name = 0;
                    }
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind"))
                        object.kind = options.enums === $String ? $root.doramcap.activity.v1.Record.Kind[message.kind] === $undefined ? message.kind : $root.doramcap.activity.v1.Record.Kind[message.kind] : message.kind;
                    if (message.monotonicNs != null && $Object.hasOwnProperty.call(message, "monotonicNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.monotonicNs = typeof message.monotonicNs === "number" ? $BigInt(message.monotonicNs) : $util.Long.fromBits(message.monotonicNs.low >>> 0, message.monotonicNs.high >>> 0, true).toBigInt();
                        else if (typeof message.monotonicNs === "number")
                            object.monotonicNs = options.longs === $String ? $String(message.monotonicNs) : message.monotonicNs;
                        else
                            object.monotonicNs = options.longs === $String ? $util.Long.prototype.toString.call(message.monotonicNs) : options.longs === $Number ? new $util.LongBits(message.monotonicNs.low >>> 0, message.monotonicNs.high >>> 0).toNumber(true) : message.monotonicNs;
                    if (message.spanId != null && $Object.hasOwnProperty.call(message, "spanId"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.spanId = typeof message.spanId === "number" ? $BigInt(message.spanId) : $util.Long.fromBits(message.spanId.low >>> 0, message.spanId.high >>> 0, true).toBigInt();
                        else if (typeof message.spanId === "number")
                            object.spanId = options.longs === $String ? $String(message.spanId) : message.spanId;
                        else
                            object.spanId = options.longs === $String ? $util.Long.prototype.toString.call(message.spanId) : options.longs === $Number ? new $util.LongBits(message.spanId.low >>> 0, message.spanId.high >>> 0).toNumber(true) : message.spanId;
                    if (message.traceRoot != null && $Object.hasOwnProperty.call(message, "traceRoot"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.traceRoot = typeof message.traceRoot === "number" ? $BigInt(message.traceRoot) : $util.Long.fromBits(message.traceRoot.low >>> 0, message.traceRoot.high >>> 0, true).toBigInt();
                        else if (typeof message.traceRoot === "number")
                            object.traceRoot = options.longs === $String ? $String(message.traceRoot) : message.traceRoot;
                        else
                            object.traceRoot = options.longs === $String ? $util.Long.prototype.toString.call(message.traceRoot) : options.longs === $Number ? new $util.LongBits(message.traceRoot.low >>> 0, message.traceRoot.high >>> 0).toNumber(true) : message.traceRoot;
                    if (message.parentId != null && $Object.hasOwnProperty.call(message, "parentId"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.parentId = typeof message.parentId === "number" ? $BigInt(message.parentId) : $util.Long.fromBits(message.parentId.low >>> 0, message.parentId.high >>> 0, true).toBigInt();
                        else if (typeof message.parentId === "number")
                            object.parentId = options.longs === $String ? $String(message.parentId) : message.parentId;
                        else
                            object.parentId = options.longs === $String ? $util.Long.prototype.toString.call(message.parentId) : options.longs === $Number ? new $util.LongBits(message.parentId.low >>> 0, message.parentId.high >>> 0).toNumber(true) : message.parentId;
                    if (message.linkedId != null && $Object.hasOwnProperty.call(message, "linkedId"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.linkedId = typeof message.linkedId === "number" ? $BigInt(message.linkedId) : $util.Long.fromBits(message.linkedId.low >>> 0, message.linkedId.high >>> 0, true).toBigInt();
                        else if (typeof message.linkedId === "number")
                            object.linkedId = options.longs === $String ? $String(message.linkedId) : message.linkedId;
                        else
                            object.linkedId = options.longs === $String ? $util.Long.prototype.toString.call(message.linkedId) : options.longs === $Number ? new $util.LongBits(message.linkedId.low >>> 0, message.linkedId.high >>> 0).toNumber(true) : message.linkedId;
                    if (message.linkedRoot != null && $Object.hasOwnProperty.call(message, "linkedRoot"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.linkedRoot = typeof message.linkedRoot === "number" ? $BigInt(message.linkedRoot) : $util.Long.fromBits(message.linkedRoot.low >>> 0, message.linkedRoot.high >>> 0, true).toBigInt();
                        else if (typeof message.linkedRoot === "number")
                            object.linkedRoot = options.longs === $String ? $String(message.linkedRoot) : message.linkedRoot;
                        else
                            object.linkedRoot = options.longs === $String ? $util.Long.prototype.toString.call(message.linkedRoot) : options.longs === $Number ? new $util.LongBits(message.linkedRoot.low >>> 0, message.linkedRoot.high >>> 0).toNumber(true) : message.linkedRoot;
                    if (message.tid != null && $Object.hasOwnProperty.call(message, "tid"))
                        object.tid = message.tid;
                    if (message.threadName != null && $Object.hasOwnProperty.call(message, "threadName"))
                        object.threadName = message.threadName;
                    if (message.name != null && $Object.hasOwnProperty.call(message, "name"))
                        object.name = message.name;
                    if (message.fields && message.fields.length) {
                        object.fields = $Array(message.fields.length);
                        for (let j = 0; j < message.fields.length; ++j)
                            object.fields[j] = $root.doramcap.activity.v1.Field.toObject(message.fields[j], options, _depth + 1);
                    }
                    return object;
                };

                /**
                 * Converts this Record to JSON.
                 * @function toJSON
                 * @memberof doramcap.activity.v1.Record
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Record.prototype.toJSON = function() {
                    return Record.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Record
                 * @function getTypeUrl
                 * @memberof doramcap.activity.v1.Record
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Record.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.activity.v1.Record";
                };

                /**
                 * Kind enum.
                 * @name doramcap.activity.v1.Record.Kind
                 * @enum {number}
                 * @property {number} BEGIN=0 BEGIN value
                 * @property {number} EVENT=1 EVENT value
                 * @property {number} UPDATE=2 UPDATE value
                 * @property {number} LINK=3 LINK value
                 * @property {number} END=4 END value
                 */
                Record.Kind = (function() {
                    const valuesById = $Object.create(null), values = $Object.create(valuesById);
                    values[valuesById[0] = "BEGIN"] = 0;
                    values[valuesById[1] = "EVENT"] = 1;
                    values[valuesById[2] = "UPDATE"] = 2;
                    values[valuesById[3] = "LINK"] = 3;
                    values[valuesById[4] = "END"] = 4;
                    return values;
                })();

                return Record;
            })();

            v1.Field = (function() {

                /**
                 * Properties of a Field.
                 * @typedef {Object} doramcap.activity.v1.Field.$Properties
                 * @property {number|null} [name] Field name
                 * @property {doramcap.activity.v1.Field.Kind|null} [kind] Field kind
                 * @property {number|Long|null} [bits] Field bits
                 * @property {number|null} [offset] Field offset
                 * @property {number|null} [length] Field length
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Field.
                 * @memberof doramcap.activity.v1
                 * @interface IField
                 * @augments doramcap.activity.v1.Field.$Properties
                 * @deprecated Use doramcap.activity.v1.Field.$Properties instead.
                 */

                /**
                 * Shape of a Field.
                 * @typedef {doramcap.activity.v1.Field.$Properties} doramcap.activity.v1.Field.$Shape
                 */

                /**
                 * Constructs a new Field.
                 * @memberof doramcap.activity.v1
                 * @classdesc Represents a Field.
                 * @constructor
                 * @param {doramcap.activity.v1.Field.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Field = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Field name.
                 * @member {number} name
                 * @memberof doramcap.activity.v1.Field
                 * @instance
                 */
                Field.prototype.name = 0;

                /**
                 * Field kind.
                 * @member {doramcap.activity.v1.Field.Kind} kind
                 * @memberof doramcap.activity.v1.Field
                 * @instance
                 */
                Field.prototype.kind = 0;

                /**
                 * Field bits.
                 * @member {number|Long} bits
                 * @memberof doramcap.activity.v1.Field
                 * @instance
                 */
                Field.prototype.bits = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Field offset.
                 * @member {number} offset
                 * @memberof doramcap.activity.v1.Field
                 * @instance
                 */
                Field.prototype.offset = 0;

                /**
                 * Field length.
                 * @member {number} length
                 * @memberof doramcap.activity.v1.Field
                 * @instance
                 */
                Field.prototype.length = 0;

                /**
                 * Encodes the specified Field message. Does not implicitly {@link doramcap.activity.v1.Field.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.activity.v1.Field
                 * @static
                 * @param {doramcap.activity.v1.Field.$Properties} message Field message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Field.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.name != null && $Object.hasOwnProperty.call(message, "name") && message.name !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.name);
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind") && message.kind !== 0)
                        writer.uint32(/* id 2, wireType 0 =*/16).int32(message.kind);
                    if (message.bits != null && $Object.hasOwnProperty.call(message, "bits") && (typeof message.bits === "object" ? message.bits.low || message.bits.high : message.bits !== 0))
                        writer.uint32(/* id 3, wireType 1 =*/25).fixed64(message.bits);
                    if (message.offset != null && $Object.hasOwnProperty.call(message, "offset") && message.offset !== 0)
                        writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.offset);
                    if (message.length != null && $Object.hasOwnProperty.call(message, "length") && message.length !== 0)
                        writer.uint32(/* id 5, wireType 0 =*/40).uint32(message.length);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Field message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.activity.v1.Field
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.activity.v1.Field & doramcap.activity.v1.Field.$Shape} Field
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Field.decode = function (reader, length, _end, _depth, _target) {
                    if (!(reader instanceof $Reader))
                        reader = $Reader.create(reader);
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $Reader.recursionLimit)
                        throw $Error("max depth exceeded");
                    let end, message, value;
                    if (length === $undefined)
                        end = reader.len;
                    else {
                        end = reader.pos + length;
                        if (end > reader.len)
                            throw $RangeError("index out of range");
                        length = reader.len;
                        reader.len = end;
                    }
                    message = _target || new $root.doramcap.activity.v1.Field();
                    while (reader.pos < end) {
                        let start = reader.pos;
                        let tag = reader.tag();
                        if (tag === _end) {
                            _end = $undefined;
                            break;
                        }
                        let wireType = tag & 7;
                        switch (tag >>>= 3) {
                        case 1: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.name = value;
                                else
                                    delete message.name;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.int32())
                                    message.kind = value;
                                else
                                    delete message.kind;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 1)
                                    break;
                                if (typeof (value = reader.fixed64()) === "object" ? value.low || value.high : value !== 0)
                                    message.bits = value;
                                else
                                    delete message.bits;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.offset = value;
                                else
                                    delete message.offset;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.length = value;
                                else
                                    delete message.length;
                                continue;
                            }
                        }
                        reader.skipType(wireType, _depth, tag);
                        if (!reader.discardUnknown) {
                            $util.makeProp(message, "$unknowns", false);
                            (message.$unknowns || (message.$unknowns = [])).push(reader.raw(start, reader.pos));
                        }
                    }
                    if (length !== $undefined) {
                        if (reader.pos !== end)
                            throw $RangeError("index out of range");
                        reader.len = length;
                    }
                    if (_end !== $undefined)
                        throw $Error("missing end group");
                    return message;
                };

                /**
                 * Creates a Field message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.activity.v1.Field
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.activity.v1.Field} Field
                 */
                Field.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.activity.v1.Field)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.activity.v1.Field: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.activity.v1.Field();
                    if (object.name != null)
                        if ($Number(object.name) !== 0)
                            message.name = object.name >>> 0;
                    if (object.kind !== 0 && (typeof object.kind !== "string" || $root.doramcap.activity.v1.Field.Kind[object.kind] !== 0))
                        switch (object.kind) {
                        case "UNSIGNED":
                        case 0:
                            message.kind = 0;
                            break;
                        case "SIGNED":
                        case 1:
                            message.kind = 1;
                            break;
                        case "FLOAT":
                        case 2:
                            message.kind = 2;
                            break;
                        case "BOOL":
                        case 3:
                            message.kind = 3;
                            break;
                        case "STRING":
                        case 4:
                            message.kind = 4;
                            break;
                        default:
                            if (typeof object.kind === "number" && (object.kind | 0) === object.kind)
                                message.kind = object.kind;
                        }
                    if (object.bits != null)
                        if (typeof object.bits === "object" ? object.bits.low || object.bits.high : $Number(object.bits) !== 0)
                            if ($util.Long)
                                message.bits = $util.Long.fromValue(object.bits, true);
                            else if (typeof object.bits === "string")
                                message.bits = $parseInt(object.bits, 10);
                            else if (typeof object.bits === "number")
                                message.bits = object.bits;
                            else if (typeof object.bits === "object")
                                message.bits = new $util.LongBits(object.bits.low >>> 0, object.bits.high >>> 0).toNumber(true);
                    if (object.offset != null)
                        if ($Number(object.offset) !== 0)
                            message.offset = object.offset >>> 0;
                    if (object.length != null)
                        if ($Number(object.length) !== 0)
                            message.length = object.length >>> 0;
                    return message;
                };

                /**
                 * Creates a plain object from a Field message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.activity.v1.Field
                 * @static
                 * @param {doramcap.activity.v1.Field} message Field
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Field.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.name = 0;
                        object.kind = options.enums === $String ? "UNSIGNED" : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.bits = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.bits = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.offset = 0;
                        object.length = 0;
                    }
                    if (message.name != null && $Object.hasOwnProperty.call(message, "name"))
                        object.name = message.name;
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind"))
                        object.kind = options.enums === $String ? $root.doramcap.activity.v1.Field.Kind[message.kind] === $undefined ? message.kind : $root.doramcap.activity.v1.Field.Kind[message.kind] : message.kind;
                    if (message.bits != null && $Object.hasOwnProperty.call(message, "bits"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.bits = typeof message.bits === "number" ? $BigInt(message.bits) : $util.Long.fromBits(message.bits.low >>> 0, message.bits.high >>> 0, true).toBigInt();
                        else if (typeof message.bits === "number")
                            object.bits = options.longs === $String ? $String(message.bits) : message.bits;
                        else
                            object.bits = options.longs === $String ? $util.Long.prototype.toString.call(message.bits) : options.longs === $Number ? new $util.LongBits(message.bits.low >>> 0, message.bits.high >>> 0).toNumber(true) : message.bits;
                    if (message.offset != null && $Object.hasOwnProperty.call(message, "offset"))
                        object.offset = message.offset;
                    if (message.length != null && $Object.hasOwnProperty.call(message, "length"))
                        object.length = message.length;
                    return object;
                };

                /**
                 * Converts this Field to JSON.
                 * @function toJSON
                 * @memberof doramcap.activity.v1.Field
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Field.prototype.toJSON = function() {
                    return Field.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Field
                 * @function getTypeUrl
                 * @memberof doramcap.activity.v1.Field
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Field.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.activity.v1.Field";
                };

                /**
                 * Kind enum.
                 * @name doramcap.activity.v1.Field.Kind
                 * @enum {number}
                 * @property {number} UNSIGNED=0 UNSIGNED value
                 * @property {number} SIGNED=1 SIGNED value
                 * @property {number} FLOAT=2 FLOAT value
                 * @property {number} BOOL=3 BOOL value
                 * @property {number} STRING=4 STRING value
                 */
                Field.Kind = (function() {
                    const valuesById = $Object.create(null), values = $Object.create(valuesById);
                    values[valuesById[0] = "UNSIGNED"] = 0;
                    values[valuesById[1] = "SIGNED"] = 1;
                    values[valuesById[2] = "FLOAT"] = 2;
                    values[valuesById[3] = "BOOL"] = 3;
                    values[valuesById[4] = "STRING"] = 4;
                    return values;
                })();

                return Field;
            })();

            return v1;
        })();

        return activity;
    })();

    return doramcap;
})();

export {
  $root as default
};
