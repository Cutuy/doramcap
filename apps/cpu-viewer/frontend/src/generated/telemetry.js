/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-mixed-operators, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars, default-case, jsdoc/require-param*/
import $protobuf from "protobufjs/minimal.js";

// Common aliases
const $Reader = $protobuf.Reader, $Writer = $protobuf.Writer, $util = $protobuf.util;
const $Object = $util.global.Object, $undefined = $util.global.undefined, $Error = $util.global.Error, $RangeError = $util.global.RangeError, $TypeError = $util.global.TypeError, $Number = $util.global.Number, $parseInt = $util.global.parseInt, $String = $util.global.String, $BigInt = $util.global.BigInt, $Array = $util.global.Array, $isFinite = $util.global.isFinite, $Boolean = $util.global.Boolean;

// Exported root namespace
const $root = $protobuf.roots["default"] || ($protobuf.roots["default"] = {});

export const doramcap = $root.doramcap = (() => {

    /**
     * Namespace doramcap.
     * @exports doramcap
     * @namespace
     */
    const doramcap = {};

    doramcap.telemetry = (function() {

        /**
         * Namespace telemetry.
         * @memberof doramcap
         * @namespace
         */
        const telemetry = {};

        telemetry.v1 = (function() {

            /**
             * Namespace v1.
             * @memberof doramcap.telemetry
             * @namespace
             */
            const v1 = {};

            v1.Packet = (function() {

                /**
                 * Properties of a Packet.
                 * @typedef {Object} doramcap.telemetry.v1.Packet.$Properties
                 * @property {doramcap.telemetry.v1.SnapshotFrame.$Properties|null} [snapshot] Packet snapshot
                 * @property {doramcap.telemetry.v1.DictionaryFrame.$Properties|null} [dictionary] Packet dictionary
                 * @property {"snapshot"|"dictionary"} [body] Packet body
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Packet.
                 * @memberof doramcap.telemetry.v1
                 * @interface IPacket
                 * @augments doramcap.telemetry.v1.Packet.$Properties
                 * @deprecated Use doramcap.telemetry.v1.Packet.$Properties instead.
                 */

                /**
                 * Narrowed shape of a Packet.
                 * @typedef {{
                 *   snapshot?: doramcap.telemetry.v1.SnapshotFrame.$Shape|null;
                 *   dictionary?: doramcap.telemetry.v1.DictionaryFrame.$Shape|null;
                 *   $unknowns?: Array.<Uint8Array>;
                 * } & (
                 *   ({ body?: undefined; snapshot?: null; dictionary?: null }|{ body?: "snapshot"; snapshot: doramcap.telemetry.v1.SnapshotFrame.$Shape; dictionary?: null }|{ body?: "dictionary"; snapshot?: null; dictionary: doramcap.telemetry.v1.DictionaryFrame.$Shape })
                 * )} doramcap.telemetry.v1.Packet.$Shape
                 */

                /**
                 * Constructs a new Packet.
                 * @memberof doramcap.telemetry.v1
                 * @classdesc Represents a Packet.
                 * @constructor
                 * @param {doramcap.telemetry.v1.Packet.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Packet = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Packet snapshot.
                 * @member {doramcap.telemetry.v1.SnapshotFrame.$Properties|null|undefined} snapshot
                 * @memberof doramcap.telemetry.v1.Packet
                 * @instance
                 */
                Packet.prototype.snapshot = null;

                /**
                 * Packet dictionary.
                 * @member {doramcap.telemetry.v1.DictionaryFrame.$Properties|null|undefined} dictionary
                 * @memberof doramcap.telemetry.v1.Packet
                 * @instance
                 */
                Packet.prototype.dictionary = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                /**
                 * Packet body.
                 * @member {"snapshot"|"dictionary"|undefined} body
                 * @memberof doramcap.telemetry.v1.Packet
                 * @instance
                 */
                $Object.defineProperty(Packet.prototype, "body", {
                    get: $util.oneOfGetter($oneOfFields = ["snapshot", "dictionary"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Packet message. Does not implicitly {@link doramcap.telemetry.v1.Packet.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.telemetry.v1.Packet
                 * @static
                 * @param {doramcap.telemetry.v1.Packet.$Properties} message Packet message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Packet.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.snapshot != null && $Object.hasOwnProperty.call(message, "snapshot"))
                        $root.doramcap.telemetry.v1.SnapshotFrame.encode(message.snapshot, writer.uint32(/* id 1, wireType 2 =*/10).fork(), _depth + 1).ldelim();
                    if (message.dictionary != null && $Object.hasOwnProperty.call(message, "dictionary"))
                        $root.doramcap.telemetry.v1.DictionaryFrame.encode(message.dictionary, writer.uint32(/* id 2, wireType 2 =*/18).fork(), _depth + 1).ldelim();
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Packet message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.telemetry.v1.Packet
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.Packet & doramcap.telemetry.v1.Packet.$Shape} Packet
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Packet.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.telemetry.v1.Packet();
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
                                message.snapshot = $root.doramcap.telemetry.v1.SnapshotFrame.decode(reader, reader.size(), $undefined, _depth + 1, message.snapshot);
                                message.body = "snapshot";
                                continue;
                            }
                        case 2: {
                                if (wireType !== 2)
                                    break;
                                message.dictionary = $root.doramcap.telemetry.v1.DictionaryFrame.decode(reader, reader.size(), $undefined, _depth + 1, message.dictionary);
                                message.body = "dictionary";
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
                 * Creates a Packet message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.telemetry.v1.Packet
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.telemetry.v1.Packet} Packet
                 */
                Packet.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.telemetry.v1.Packet)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.telemetry.v1.Packet: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.telemetry.v1.Packet();
                    if (object.snapshot != null) {
                        if (!$util.isObject(object.snapshot))
                            throw $TypeError(".doramcap.telemetry.v1.Packet.snapshot: object expected");
                        message.snapshot = $root.doramcap.telemetry.v1.SnapshotFrame.fromObject(object.snapshot, _depth + 1);
                    }
                    if (object.dictionary != null) {
                        if (!$util.isObject(object.dictionary))
                            throw $TypeError(".doramcap.telemetry.v1.Packet.dictionary: object expected");
                        message.dictionary = $root.doramcap.telemetry.v1.DictionaryFrame.fromObject(object.dictionary, _depth + 1);
                    }
                    return message;
                };

                /**
                 * Creates a plain object from a Packet message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.telemetry.v1.Packet
                 * @static
                 * @param {doramcap.telemetry.v1.Packet} message Packet
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Packet.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (message.snapshot != null && $Object.hasOwnProperty.call(message, "snapshot")) {
                        object.snapshot = $root.doramcap.telemetry.v1.SnapshotFrame.toObject(message.snapshot, options, _depth + 1);
                        if (options.oneofs)
                            object.body = "snapshot";
                    }
                    if (message.dictionary != null && $Object.hasOwnProperty.call(message, "dictionary")) {
                        object.dictionary = $root.doramcap.telemetry.v1.DictionaryFrame.toObject(message.dictionary, options, _depth + 1);
                        if (options.oneofs)
                            object.body = "dictionary";
                    }
                    return object;
                };

                /**
                 * Converts this Packet to JSON.
                 * @function toJSON
                 * @memberof doramcap.telemetry.v1.Packet
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Packet.prototype.toJSON = function() {
                    return Packet.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Packet
                 * @function getTypeUrl
                 * @memberof doramcap.telemetry.v1.Packet
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Packet.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.telemetry.v1.Packet";
                };

                return Packet;
            })();

            v1.CpuRecord = (function() {

                /**
                 * Properties of a CpuRecord.
                 * @typedef {Object} doramcap.telemetry.v1.CpuRecord.$Properties
                 * @property {number|Long|null} [cgroupId] CpuRecord cgroupId
                 * @property {number|Long|null} [parentId] CpuRecord parentId
                 * @property {number|null} [cpu] CpuRecord cpu
                 * @property {number|null} [flags] CpuRecord flags
                 * @property {number|Long|null} [runtimeNs] CpuRecord runtimeNs
                 * @property {number|Long|null} [userNs] CpuRecord userNs
                 * @property {number|Long|null} [systemNs] CpuRecord systemNs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a CpuRecord.
                 * @memberof doramcap.telemetry.v1
                 * @interface ICpuRecord
                 * @augments doramcap.telemetry.v1.CpuRecord.$Properties
                 * @deprecated Use doramcap.telemetry.v1.CpuRecord.$Properties instead.
                 */

                /**
                 * Shape of a CpuRecord.
                 * @typedef {doramcap.telemetry.v1.CpuRecord.$Properties} doramcap.telemetry.v1.CpuRecord.$Shape
                 */

                /**
                 * Constructs a new CpuRecord.
                 * @memberof doramcap.telemetry.v1
                 * @classdesc Represents a CpuRecord.
                 * @constructor
                 * @param {doramcap.telemetry.v1.CpuRecord.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const CpuRecord = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * CpuRecord cgroupId.
                 * @member {number|Long} cgroupId
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @instance
                 */
                CpuRecord.prototype.cgroupId = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * CpuRecord parentId.
                 * @member {number|Long} parentId
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @instance
                 */
                CpuRecord.prototype.parentId = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * CpuRecord cpu.
                 * @member {number} cpu
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @instance
                 */
                CpuRecord.prototype.cpu = 0;

                /**
                 * CpuRecord flags.
                 * @member {number} flags
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @instance
                 */
                CpuRecord.prototype.flags = 0;

                /**
                 * CpuRecord runtimeNs.
                 * @member {number|Long} runtimeNs
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @instance
                 */
                CpuRecord.prototype.runtimeNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * CpuRecord userNs.
                 * @member {number|Long} userNs
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @instance
                 */
                CpuRecord.prototype.userNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * CpuRecord systemNs.
                 * @member {number|Long} systemNs
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @instance
                 */
                CpuRecord.prototype.systemNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified CpuRecord message. Does not implicitly {@link doramcap.telemetry.v1.CpuRecord.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @static
                 * @param {doramcap.telemetry.v1.CpuRecord.$Properties} message CpuRecord message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                CpuRecord.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.cgroupId != null && $Object.hasOwnProperty.call(message, "cgroupId") && (typeof message.cgroupId === "object" ? message.cgroupId.low || message.cgroupId.high : message.cgroupId !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.cgroupId);
                    if (message.parentId != null && $Object.hasOwnProperty.call(message, "parentId") && (typeof message.parentId === "object" ? message.parentId.low || message.parentId.high : message.parentId !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.parentId);
                    if (message.cpu != null && $Object.hasOwnProperty.call(message, "cpu") && message.cpu !== 0)
                        writer.uint32(/* id 3, wireType 0 =*/24).uint32(message.cpu);
                    if (message.flags != null && $Object.hasOwnProperty.call(message, "flags") && message.flags !== 0)
                        writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.flags);
                    if (message.runtimeNs != null && $Object.hasOwnProperty.call(message, "runtimeNs") && (typeof message.runtimeNs === "object" ? message.runtimeNs.low || message.runtimeNs.high : message.runtimeNs !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.runtimeNs);
                    if (message.userNs != null && $Object.hasOwnProperty.call(message, "userNs") && (typeof message.userNs === "object" ? message.userNs.low || message.userNs.high : message.userNs !== 0))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.userNs);
                    if (message.systemNs != null && $Object.hasOwnProperty.call(message, "systemNs") && (typeof message.systemNs === "object" ? message.systemNs.low || message.systemNs.high : message.systemNs !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.systemNs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a CpuRecord message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.CpuRecord & doramcap.telemetry.v1.CpuRecord.$Shape} CpuRecord
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                CpuRecord.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.telemetry.v1.CpuRecord();
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
                                    message.cgroupId = value;
                                else
                                    delete message.cgroupId;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.parentId = value;
                                else
                                    delete message.parentId;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.cpu = value;
                                else
                                    delete message.cpu;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.flags = value;
                                else
                                    delete message.flags;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.runtimeNs = value;
                                else
                                    delete message.runtimeNs;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.userNs = value;
                                else
                                    delete message.userNs;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.systemNs = value;
                                else
                                    delete message.systemNs;
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
                 * Creates a CpuRecord message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.telemetry.v1.CpuRecord} CpuRecord
                 */
                CpuRecord.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.telemetry.v1.CpuRecord)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.telemetry.v1.CpuRecord: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.telemetry.v1.CpuRecord();
                    if (object.cgroupId != null)
                        if (typeof object.cgroupId === "object" ? object.cgroupId.low || object.cgroupId.high : $Number(object.cgroupId) !== 0)
                            if ($util.Long)
                                message.cgroupId = $util.Long.fromValue(object.cgroupId, true);
                            else if (typeof object.cgroupId === "string")
                                message.cgroupId = $parseInt(object.cgroupId, 10);
                            else if (typeof object.cgroupId === "number")
                                message.cgroupId = object.cgroupId;
                            else if (typeof object.cgroupId === "object")
                                message.cgroupId = new $util.LongBits(object.cgroupId.low >>> 0, object.cgroupId.high >>> 0).toNumber(true);
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
                    if (object.cpu != null)
                        if ($Number(object.cpu) !== 0)
                            message.cpu = object.cpu >>> 0;
                    if (object.flags != null)
                        if ($Number(object.flags) !== 0)
                            message.flags = object.flags >>> 0;
                    if (object.runtimeNs != null)
                        if (typeof object.runtimeNs === "object" ? object.runtimeNs.low || object.runtimeNs.high : $Number(object.runtimeNs) !== 0)
                            if ($util.Long)
                                message.runtimeNs = $util.Long.fromValue(object.runtimeNs, true);
                            else if (typeof object.runtimeNs === "string")
                                message.runtimeNs = $parseInt(object.runtimeNs, 10);
                            else if (typeof object.runtimeNs === "number")
                                message.runtimeNs = object.runtimeNs;
                            else if (typeof object.runtimeNs === "object")
                                message.runtimeNs = new $util.LongBits(object.runtimeNs.low >>> 0, object.runtimeNs.high >>> 0).toNumber(true);
                    if (object.userNs != null)
                        if (typeof object.userNs === "object" ? object.userNs.low || object.userNs.high : $Number(object.userNs) !== 0)
                            if ($util.Long)
                                message.userNs = $util.Long.fromValue(object.userNs, true);
                            else if (typeof object.userNs === "string")
                                message.userNs = $parseInt(object.userNs, 10);
                            else if (typeof object.userNs === "number")
                                message.userNs = object.userNs;
                            else if (typeof object.userNs === "object")
                                message.userNs = new $util.LongBits(object.userNs.low >>> 0, object.userNs.high >>> 0).toNumber(true);
                    if (object.systemNs != null)
                        if (typeof object.systemNs === "object" ? object.systemNs.low || object.systemNs.high : $Number(object.systemNs) !== 0)
                            if ($util.Long)
                                message.systemNs = $util.Long.fromValue(object.systemNs, true);
                            else if (typeof object.systemNs === "string")
                                message.systemNs = $parseInt(object.systemNs, 10);
                            else if (typeof object.systemNs === "number")
                                message.systemNs = object.systemNs;
                            else if (typeof object.systemNs === "object")
                                message.systemNs = new $util.LongBits(object.systemNs.low >>> 0, object.systemNs.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a CpuRecord message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @static
                 * @param {doramcap.telemetry.v1.CpuRecord} message CpuRecord
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                CpuRecord.toObject = function (message, options, _depth) {
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
                            object.cgroupId = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.cgroupId = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.parentId = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.parentId = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.cpu = 0;
                        object.flags = 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.runtimeNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.runtimeNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.userNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.userNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.systemNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.systemNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.cgroupId != null && $Object.hasOwnProperty.call(message, "cgroupId"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.cgroupId = typeof message.cgroupId === "number" ? $BigInt(message.cgroupId) : $util.Long.fromBits(message.cgroupId.low >>> 0, message.cgroupId.high >>> 0, true).toBigInt();
                        else if (typeof message.cgroupId === "number")
                            object.cgroupId = options.longs === $String ? $String(message.cgroupId) : message.cgroupId;
                        else
                            object.cgroupId = options.longs === $String ? $util.Long.prototype.toString.call(message.cgroupId) : options.longs === $Number ? new $util.LongBits(message.cgroupId.low >>> 0, message.cgroupId.high >>> 0).toNumber(true) : message.cgroupId;
                    if (message.parentId != null && $Object.hasOwnProperty.call(message, "parentId"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.parentId = typeof message.parentId === "number" ? $BigInt(message.parentId) : $util.Long.fromBits(message.parentId.low >>> 0, message.parentId.high >>> 0, true).toBigInt();
                        else if (typeof message.parentId === "number")
                            object.parentId = options.longs === $String ? $String(message.parentId) : message.parentId;
                        else
                            object.parentId = options.longs === $String ? $util.Long.prototype.toString.call(message.parentId) : options.longs === $Number ? new $util.LongBits(message.parentId.low >>> 0, message.parentId.high >>> 0).toNumber(true) : message.parentId;
                    if (message.cpu != null && $Object.hasOwnProperty.call(message, "cpu"))
                        object.cpu = message.cpu;
                    if (message.flags != null && $Object.hasOwnProperty.call(message, "flags"))
                        object.flags = message.flags;
                    if (message.runtimeNs != null && $Object.hasOwnProperty.call(message, "runtimeNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.runtimeNs = typeof message.runtimeNs === "number" ? $BigInt(message.runtimeNs) : $util.Long.fromBits(message.runtimeNs.low >>> 0, message.runtimeNs.high >>> 0, true).toBigInt();
                        else if (typeof message.runtimeNs === "number")
                            object.runtimeNs = options.longs === $String ? $String(message.runtimeNs) : message.runtimeNs;
                        else
                            object.runtimeNs = options.longs === $String ? $util.Long.prototype.toString.call(message.runtimeNs) : options.longs === $Number ? new $util.LongBits(message.runtimeNs.low >>> 0, message.runtimeNs.high >>> 0).toNumber(true) : message.runtimeNs;
                    if (message.userNs != null && $Object.hasOwnProperty.call(message, "userNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.userNs = typeof message.userNs === "number" ? $BigInt(message.userNs) : $util.Long.fromBits(message.userNs.low >>> 0, message.userNs.high >>> 0, true).toBigInt();
                        else if (typeof message.userNs === "number")
                            object.userNs = options.longs === $String ? $String(message.userNs) : message.userNs;
                        else
                            object.userNs = options.longs === $String ? $util.Long.prototype.toString.call(message.userNs) : options.longs === $Number ? new $util.LongBits(message.userNs.low >>> 0, message.userNs.high >>> 0).toNumber(true) : message.userNs;
                    if (message.systemNs != null && $Object.hasOwnProperty.call(message, "systemNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.systemNs = typeof message.systemNs === "number" ? $BigInt(message.systemNs) : $util.Long.fromBits(message.systemNs.low >>> 0, message.systemNs.high >>> 0, true).toBigInt();
                        else if (typeof message.systemNs === "number")
                            object.systemNs = options.longs === $String ? $String(message.systemNs) : message.systemNs;
                        else
                            object.systemNs = options.longs === $String ? $util.Long.prototype.toString.call(message.systemNs) : options.longs === $Number ? new $util.LongBits(message.systemNs.low >>> 0, message.systemNs.high >>> 0).toNumber(true) : message.systemNs;
                    return object;
                };

                /**
                 * Converts this CpuRecord to JSON.
                 * @function toJSON
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                CpuRecord.prototype.toJSON = function() {
                    return CpuRecord.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for CpuRecord
                 * @function getTypeUrl
                 * @memberof doramcap.telemetry.v1.CpuRecord
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                CpuRecord.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.telemetry.v1.CpuRecord";
                };

                return CpuRecord;
            })();

            v1.TaskIdentity = (function() {

                /**
                 * Properties of a TaskIdentity.
                 * @typedef {Object} doramcap.telemetry.v1.TaskIdentity.$Properties
                 * @property {number|Long|null} [startBoottimeNs] TaskIdentity startBoottimeNs
                 * @property {number|Long|null} [cgroupId] TaskIdentity cgroupId
                 * @property {number|null} [tid] TaskIdentity tid
                 * @property {number|null} [tgid] TaskIdentity tgid
                 * @property {number|null} [cpu] TaskIdentity cpu
                 * @property {number|null} [flags] TaskIdentity flags
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a TaskIdentity.
                 * @memberof doramcap.telemetry.v1
                 * @interface ITaskIdentity
                 * @augments doramcap.telemetry.v1.TaskIdentity.$Properties
                 * @deprecated Use doramcap.telemetry.v1.TaskIdentity.$Properties instead.
                 */

                /**
                 * Shape of a TaskIdentity.
                 * @typedef {doramcap.telemetry.v1.TaskIdentity.$Properties} doramcap.telemetry.v1.TaskIdentity.$Shape
                 */

                /**
                 * Constructs a new TaskIdentity.
                 * @memberof doramcap.telemetry.v1
                 * @classdesc Represents a TaskIdentity.
                 * @constructor
                 * @param {doramcap.telemetry.v1.TaskIdentity.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const TaskIdentity = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * TaskIdentity startBoottimeNs.
                 * @member {number|Long} startBoottimeNs
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @instance
                 */
                TaskIdentity.prototype.startBoottimeNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * TaskIdentity cgroupId.
                 * @member {number|Long} cgroupId
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @instance
                 */
                TaskIdentity.prototype.cgroupId = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * TaskIdentity tid.
                 * @member {number} tid
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @instance
                 */
                TaskIdentity.prototype.tid = 0;

                /**
                 * TaskIdentity tgid.
                 * @member {number} tgid
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @instance
                 */
                TaskIdentity.prototype.tgid = 0;

                /**
                 * TaskIdentity cpu.
                 * @member {number} cpu
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @instance
                 */
                TaskIdentity.prototype.cpu = 0;

                /**
                 * TaskIdentity flags.
                 * @member {number} flags
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @instance
                 */
                TaskIdentity.prototype.flags = 0;

                /**
                 * Encodes the specified TaskIdentity message. Does not implicitly {@link doramcap.telemetry.v1.TaskIdentity.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @static
                 * @param {doramcap.telemetry.v1.TaskIdentity.$Properties} message TaskIdentity message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                TaskIdentity.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.startBoottimeNs != null && $Object.hasOwnProperty.call(message, "startBoottimeNs") && (typeof message.startBoottimeNs === "object" ? message.startBoottimeNs.low || message.startBoottimeNs.high : message.startBoottimeNs !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.startBoottimeNs);
                    if (message.cgroupId != null && $Object.hasOwnProperty.call(message, "cgroupId") && (typeof message.cgroupId === "object" ? message.cgroupId.low || message.cgroupId.high : message.cgroupId !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.cgroupId);
                    if (message.tid != null && $Object.hasOwnProperty.call(message, "tid") && message.tid !== 0)
                        writer.uint32(/* id 3, wireType 0 =*/24).uint32(message.tid);
                    if (message.tgid != null && $Object.hasOwnProperty.call(message, "tgid") && message.tgid !== 0)
                        writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.tgid);
                    if (message.cpu != null && $Object.hasOwnProperty.call(message, "cpu") && message.cpu !== 0)
                        writer.uint32(/* id 5, wireType 0 =*/40).uint32(message.cpu);
                    if (message.flags != null && $Object.hasOwnProperty.call(message, "flags") && message.flags !== 0)
                        writer.uint32(/* id 6, wireType 0 =*/48).uint32(message.flags);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a TaskIdentity message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.TaskIdentity & doramcap.telemetry.v1.TaskIdentity.$Shape} TaskIdentity
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                TaskIdentity.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.telemetry.v1.TaskIdentity();
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
                                    message.startBoottimeNs = value;
                                else
                                    delete message.startBoottimeNs;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.cgroupId = value;
                                else
                                    delete message.cgroupId;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.tid = value;
                                else
                                    delete message.tid;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.tgid = value;
                                else
                                    delete message.tgid;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.cpu = value;
                                else
                                    delete message.cpu;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.flags = value;
                                else
                                    delete message.flags;
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
                 * Creates a TaskIdentity message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.telemetry.v1.TaskIdentity} TaskIdentity
                 */
                TaskIdentity.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.telemetry.v1.TaskIdentity)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.telemetry.v1.TaskIdentity: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.telemetry.v1.TaskIdentity();
                    if (object.startBoottimeNs != null)
                        if (typeof object.startBoottimeNs === "object" ? object.startBoottimeNs.low || object.startBoottimeNs.high : $Number(object.startBoottimeNs) !== 0)
                            if ($util.Long)
                                message.startBoottimeNs = $util.Long.fromValue(object.startBoottimeNs, true);
                            else if (typeof object.startBoottimeNs === "string")
                                message.startBoottimeNs = $parseInt(object.startBoottimeNs, 10);
                            else if (typeof object.startBoottimeNs === "number")
                                message.startBoottimeNs = object.startBoottimeNs;
                            else if (typeof object.startBoottimeNs === "object")
                                message.startBoottimeNs = new $util.LongBits(object.startBoottimeNs.low >>> 0, object.startBoottimeNs.high >>> 0).toNumber(true);
                    if (object.cgroupId != null)
                        if (typeof object.cgroupId === "object" ? object.cgroupId.low || object.cgroupId.high : $Number(object.cgroupId) !== 0)
                            if ($util.Long)
                                message.cgroupId = $util.Long.fromValue(object.cgroupId, true);
                            else if (typeof object.cgroupId === "string")
                                message.cgroupId = $parseInt(object.cgroupId, 10);
                            else if (typeof object.cgroupId === "number")
                                message.cgroupId = object.cgroupId;
                            else if (typeof object.cgroupId === "object")
                                message.cgroupId = new $util.LongBits(object.cgroupId.low >>> 0, object.cgroupId.high >>> 0).toNumber(true);
                    if (object.tid != null)
                        if ($Number(object.tid) !== 0)
                            message.tid = object.tid >>> 0;
                    if (object.tgid != null)
                        if ($Number(object.tgid) !== 0)
                            message.tgid = object.tgid >>> 0;
                    if (object.cpu != null)
                        if ($Number(object.cpu) !== 0)
                            message.cpu = object.cpu >>> 0;
                    if (object.flags != null)
                        if ($Number(object.flags) !== 0)
                            message.flags = object.flags >>> 0;
                    return message;
                };

                /**
                 * Creates a plain object from a TaskIdentity message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @static
                 * @param {doramcap.telemetry.v1.TaskIdentity} message TaskIdentity
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                TaskIdentity.toObject = function (message, options, _depth) {
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
                            object.startBoottimeNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.startBoottimeNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.cgroupId = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.cgroupId = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.tid = 0;
                        object.tgid = 0;
                        object.cpu = 0;
                        object.flags = 0;
                    }
                    if (message.startBoottimeNs != null && $Object.hasOwnProperty.call(message, "startBoottimeNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.startBoottimeNs = typeof message.startBoottimeNs === "number" ? $BigInt(message.startBoottimeNs) : $util.Long.fromBits(message.startBoottimeNs.low >>> 0, message.startBoottimeNs.high >>> 0, true).toBigInt();
                        else if (typeof message.startBoottimeNs === "number")
                            object.startBoottimeNs = options.longs === $String ? $String(message.startBoottimeNs) : message.startBoottimeNs;
                        else
                            object.startBoottimeNs = options.longs === $String ? $util.Long.prototype.toString.call(message.startBoottimeNs) : options.longs === $Number ? new $util.LongBits(message.startBoottimeNs.low >>> 0, message.startBoottimeNs.high >>> 0).toNumber(true) : message.startBoottimeNs;
                    if (message.cgroupId != null && $Object.hasOwnProperty.call(message, "cgroupId"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.cgroupId = typeof message.cgroupId === "number" ? $BigInt(message.cgroupId) : $util.Long.fromBits(message.cgroupId.low >>> 0, message.cgroupId.high >>> 0, true).toBigInt();
                        else if (typeof message.cgroupId === "number")
                            object.cgroupId = options.longs === $String ? $String(message.cgroupId) : message.cgroupId;
                        else
                            object.cgroupId = options.longs === $String ? $util.Long.prototype.toString.call(message.cgroupId) : options.longs === $Number ? new $util.LongBits(message.cgroupId.low >>> 0, message.cgroupId.high >>> 0).toNumber(true) : message.cgroupId;
                    if (message.tid != null && $Object.hasOwnProperty.call(message, "tid"))
                        object.tid = message.tid;
                    if (message.tgid != null && $Object.hasOwnProperty.call(message, "tgid"))
                        object.tgid = message.tgid;
                    if (message.cpu != null && $Object.hasOwnProperty.call(message, "cpu"))
                        object.cpu = message.cpu;
                    if (message.flags != null && $Object.hasOwnProperty.call(message, "flags"))
                        object.flags = message.flags;
                    return object;
                };

                /**
                 * Converts this TaskIdentity to JSON.
                 * @function toJSON
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                TaskIdentity.prototype.toJSON = function() {
                    return TaskIdentity.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for TaskIdentity
                 * @function getTypeUrl
                 * @memberof doramcap.telemetry.v1.TaskIdentity
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                TaskIdentity.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.telemetry.v1.TaskIdentity";
                };

                return TaskIdentity;
            })();

            v1.TaskRecord = (function() {

                /**
                 * Properties of a TaskRecord.
                 * @typedef {Object} doramcap.telemetry.v1.TaskRecord.$Properties
                 * @property {doramcap.telemetry.v1.TaskIdentity.$Properties|null} [identity] TaskRecord identity
                 * @property {number|Long|null} [chargedRuntimeNs] TaskRecord chargedRuntimeNs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a TaskRecord.
                 * @memberof doramcap.telemetry.v1
                 * @interface ITaskRecord
                 * @augments doramcap.telemetry.v1.TaskRecord.$Properties
                 * @deprecated Use doramcap.telemetry.v1.TaskRecord.$Properties instead.
                 */

                /**
                 * Shape of a TaskRecord.
                 * @typedef {doramcap.telemetry.v1.TaskRecord.$Properties} doramcap.telemetry.v1.TaskRecord.$Shape
                 */

                /**
                 * Constructs a new TaskRecord.
                 * @memberof doramcap.telemetry.v1
                 * @classdesc Represents a TaskRecord.
                 * @constructor
                 * @param {doramcap.telemetry.v1.TaskRecord.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const TaskRecord = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * TaskRecord identity.
                 * @member {doramcap.telemetry.v1.TaskIdentity.$Properties|null|undefined} identity
                 * @memberof doramcap.telemetry.v1.TaskRecord
                 * @instance
                 */
                TaskRecord.prototype.identity = null;

                /**
                 * TaskRecord chargedRuntimeNs.
                 * @member {number|Long} chargedRuntimeNs
                 * @memberof doramcap.telemetry.v1.TaskRecord
                 * @instance
                 */
                TaskRecord.prototype.chargedRuntimeNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified TaskRecord message. Does not implicitly {@link doramcap.telemetry.v1.TaskRecord.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.telemetry.v1.TaskRecord
                 * @static
                 * @param {doramcap.telemetry.v1.TaskRecord.$Properties} message TaskRecord message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                TaskRecord.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.identity != null && $Object.hasOwnProperty.call(message, "identity"))
                        $root.doramcap.telemetry.v1.TaskIdentity.encode(message.identity, writer.uint32(/* id 1, wireType 2 =*/10).fork(), _depth + 1).ldelim();
                    if (message.chargedRuntimeNs != null && $Object.hasOwnProperty.call(message, "chargedRuntimeNs") && (typeof message.chargedRuntimeNs === "object" ? message.chargedRuntimeNs.low || message.chargedRuntimeNs.high : message.chargedRuntimeNs !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.chargedRuntimeNs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a TaskRecord message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.telemetry.v1.TaskRecord
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.TaskRecord & doramcap.telemetry.v1.TaskRecord.$Shape} TaskRecord
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                TaskRecord.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.telemetry.v1.TaskRecord();
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
                                message.identity = $root.doramcap.telemetry.v1.TaskIdentity.decode(reader, reader.size(), $undefined, _depth + 1, message.identity);
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.chargedRuntimeNs = value;
                                else
                                    delete message.chargedRuntimeNs;
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
                 * Creates a TaskRecord message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.telemetry.v1.TaskRecord
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.telemetry.v1.TaskRecord} TaskRecord
                 */
                TaskRecord.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.telemetry.v1.TaskRecord)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.telemetry.v1.TaskRecord: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.telemetry.v1.TaskRecord();
                    if (object.identity != null) {
                        if (!$util.isObject(object.identity))
                            throw $TypeError(".doramcap.telemetry.v1.TaskRecord.identity: object expected");
                        message.identity = $root.doramcap.telemetry.v1.TaskIdentity.fromObject(object.identity, _depth + 1);
                    }
                    if (object.chargedRuntimeNs != null)
                        if (typeof object.chargedRuntimeNs === "object" ? object.chargedRuntimeNs.low || object.chargedRuntimeNs.high : $Number(object.chargedRuntimeNs) !== 0)
                            if ($util.Long)
                                message.chargedRuntimeNs = $util.Long.fromValue(object.chargedRuntimeNs, true);
                            else if (typeof object.chargedRuntimeNs === "string")
                                message.chargedRuntimeNs = $parseInt(object.chargedRuntimeNs, 10);
                            else if (typeof object.chargedRuntimeNs === "number")
                                message.chargedRuntimeNs = object.chargedRuntimeNs;
                            else if (typeof object.chargedRuntimeNs === "object")
                                message.chargedRuntimeNs = new $util.LongBits(object.chargedRuntimeNs.low >>> 0, object.chargedRuntimeNs.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a TaskRecord message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.telemetry.v1.TaskRecord
                 * @static
                 * @param {doramcap.telemetry.v1.TaskRecord} message TaskRecord
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                TaskRecord.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.identity = null;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.chargedRuntimeNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.chargedRuntimeNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.identity != null && $Object.hasOwnProperty.call(message, "identity"))
                        object.identity = $root.doramcap.telemetry.v1.TaskIdentity.toObject(message.identity, options, _depth + 1);
                    if (message.chargedRuntimeNs != null && $Object.hasOwnProperty.call(message, "chargedRuntimeNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.chargedRuntimeNs = typeof message.chargedRuntimeNs === "number" ? $BigInt(message.chargedRuntimeNs) : $util.Long.fromBits(message.chargedRuntimeNs.low >>> 0, message.chargedRuntimeNs.high >>> 0, true).toBigInt();
                        else if (typeof message.chargedRuntimeNs === "number")
                            object.chargedRuntimeNs = options.longs === $String ? $String(message.chargedRuntimeNs) : message.chargedRuntimeNs;
                        else
                            object.chargedRuntimeNs = options.longs === $String ? $util.Long.prototype.toString.call(message.chargedRuntimeNs) : options.longs === $Number ? new $util.LongBits(message.chargedRuntimeNs.low >>> 0, message.chargedRuntimeNs.high >>> 0).toNumber(true) : message.chargedRuntimeNs;
                    return object;
                };

                /**
                 * Converts this TaskRecord to JSON.
                 * @function toJSON
                 * @memberof doramcap.telemetry.v1.TaskRecord
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                TaskRecord.prototype.toJSON = function() {
                    return TaskRecord.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for TaskRecord
                 * @function getTypeUrl
                 * @memberof doramcap.telemetry.v1.TaskRecord
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                TaskRecord.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.telemetry.v1.TaskRecord";
                };

                return TaskRecord;
            })();

            v1.TaskLosses = (function() {

                /**
                 * Properties of a TaskLosses.
                 * @typedef {Object} doramcap.telemetry.v1.TaskLosses.$Properties
                 * @property {number|Long|null} [droppedEvents] TaskLosses droppedEvents
                 * @property {number|Long|null} [droppedRuntimeNs] TaskLosses droppedRuntimeNs
                 * @property {number|Long|null} [identityErrors] TaskLosses identityErrors
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a TaskLosses.
                 * @memberof doramcap.telemetry.v1
                 * @interface ITaskLosses
                 * @augments doramcap.telemetry.v1.TaskLosses.$Properties
                 * @deprecated Use doramcap.telemetry.v1.TaskLosses.$Properties instead.
                 */

                /**
                 * Shape of a TaskLosses.
                 * @typedef {doramcap.telemetry.v1.TaskLosses.$Properties} doramcap.telemetry.v1.TaskLosses.$Shape
                 */

                /**
                 * Constructs a new TaskLosses.
                 * @memberof doramcap.telemetry.v1
                 * @classdesc Represents a TaskLosses.
                 * @constructor
                 * @param {doramcap.telemetry.v1.TaskLosses.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const TaskLosses = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * TaskLosses droppedEvents.
                 * @member {number|Long} droppedEvents
                 * @memberof doramcap.telemetry.v1.TaskLosses
                 * @instance
                 */
                TaskLosses.prototype.droppedEvents = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * TaskLosses droppedRuntimeNs.
                 * @member {number|Long} droppedRuntimeNs
                 * @memberof doramcap.telemetry.v1.TaskLosses
                 * @instance
                 */
                TaskLosses.prototype.droppedRuntimeNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * TaskLosses identityErrors.
                 * @member {number|Long} identityErrors
                 * @memberof doramcap.telemetry.v1.TaskLosses
                 * @instance
                 */
                TaskLosses.prototype.identityErrors = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified TaskLosses message. Does not implicitly {@link doramcap.telemetry.v1.TaskLosses.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.telemetry.v1.TaskLosses
                 * @static
                 * @param {doramcap.telemetry.v1.TaskLosses.$Properties} message TaskLosses message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                TaskLosses.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.droppedEvents != null && $Object.hasOwnProperty.call(message, "droppedEvents") && (typeof message.droppedEvents === "object" ? message.droppedEvents.low || message.droppedEvents.high : message.droppedEvents !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.droppedEvents);
                    if (message.droppedRuntimeNs != null && $Object.hasOwnProperty.call(message, "droppedRuntimeNs") && (typeof message.droppedRuntimeNs === "object" ? message.droppedRuntimeNs.low || message.droppedRuntimeNs.high : message.droppedRuntimeNs !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.droppedRuntimeNs);
                    if (message.identityErrors != null && $Object.hasOwnProperty.call(message, "identityErrors") && (typeof message.identityErrors === "object" ? message.identityErrors.low || message.identityErrors.high : message.identityErrors !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.identityErrors);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a TaskLosses message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.telemetry.v1.TaskLosses
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.TaskLosses & doramcap.telemetry.v1.TaskLosses.$Shape} TaskLosses
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                TaskLosses.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.telemetry.v1.TaskLosses();
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
                                    message.droppedEvents = value;
                                else
                                    delete message.droppedEvents;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.droppedRuntimeNs = value;
                                else
                                    delete message.droppedRuntimeNs;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.identityErrors = value;
                                else
                                    delete message.identityErrors;
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
                 * Creates a TaskLosses message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.telemetry.v1.TaskLosses
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.telemetry.v1.TaskLosses} TaskLosses
                 */
                TaskLosses.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.telemetry.v1.TaskLosses)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.telemetry.v1.TaskLosses: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.telemetry.v1.TaskLosses();
                    if (object.droppedEvents != null)
                        if (typeof object.droppedEvents === "object" ? object.droppedEvents.low || object.droppedEvents.high : $Number(object.droppedEvents) !== 0)
                            if ($util.Long)
                                message.droppedEvents = $util.Long.fromValue(object.droppedEvents, true);
                            else if (typeof object.droppedEvents === "string")
                                message.droppedEvents = $parseInt(object.droppedEvents, 10);
                            else if (typeof object.droppedEvents === "number")
                                message.droppedEvents = object.droppedEvents;
                            else if (typeof object.droppedEvents === "object")
                                message.droppedEvents = new $util.LongBits(object.droppedEvents.low >>> 0, object.droppedEvents.high >>> 0).toNumber(true);
                    if (object.droppedRuntimeNs != null)
                        if (typeof object.droppedRuntimeNs === "object" ? object.droppedRuntimeNs.low || object.droppedRuntimeNs.high : $Number(object.droppedRuntimeNs) !== 0)
                            if ($util.Long)
                                message.droppedRuntimeNs = $util.Long.fromValue(object.droppedRuntimeNs, true);
                            else if (typeof object.droppedRuntimeNs === "string")
                                message.droppedRuntimeNs = $parseInt(object.droppedRuntimeNs, 10);
                            else if (typeof object.droppedRuntimeNs === "number")
                                message.droppedRuntimeNs = object.droppedRuntimeNs;
                            else if (typeof object.droppedRuntimeNs === "object")
                                message.droppedRuntimeNs = new $util.LongBits(object.droppedRuntimeNs.low >>> 0, object.droppedRuntimeNs.high >>> 0).toNumber(true);
                    if (object.identityErrors != null)
                        if (typeof object.identityErrors === "object" ? object.identityErrors.low || object.identityErrors.high : $Number(object.identityErrors) !== 0)
                            if ($util.Long)
                                message.identityErrors = $util.Long.fromValue(object.identityErrors, true);
                            else if (typeof object.identityErrors === "string")
                                message.identityErrors = $parseInt(object.identityErrors, 10);
                            else if (typeof object.identityErrors === "number")
                                message.identityErrors = object.identityErrors;
                            else if (typeof object.identityErrors === "object")
                                message.identityErrors = new $util.LongBits(object.identityErrors.low >>> 0, object.identityErrors.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a TaskLosses message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.telemetry.v1.TaskLosses
                 * @static
                 * @param {doramcap.telemetry.v1.TaskLosses} message TaskLosses
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                TaskLosses.toObject = function (message, options, _depth) {
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
                            object.droppedEvents = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.droppedEvents = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.droppedRuntimeNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.droppedRuntimeNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.identityErrors = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.identityErrors = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.droppedEvents != null && $Object.hasOwnProperty.call(message, "droppedEvents"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.droppedEvents = typeof message.droppedEvents === "number" ? $BigInt(message.droppedEvents) : $util.Long.fromBits(message.droppedEvents.low >>> 0, message.droppedEvents.high >>> 0, true).toBigInt();
                        else if (typeof message.droppedEvents === "number")
                            object.droppedEvents = options.longs === $String ? $String(message.droppedEvents) : message.droppedEvents;
                        else
                            object.droppedEvents = options.longs === $String ? $util.Long.prototype.toString.call(message.droppedEvents) : options.longs === $Number ? new $util.LongBits(message.droppedEvents.low >>> 0, message.droppedEvents.high >>> 0).toNumber(true) : message.droppedEvents;
                    if (message.droppedRuntimeNs != null && $Object.hasOwnProperty.call(message, "droppedRuntimeNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.droppedRuntimeNs = typeof message.droppedRuntimeNs === "number" ? $BigInt(message.droppedRuntimeNs) : $util.Long.fromBits(message.droppedRuntimeNs.low >>> 0, message.droppedRuntimeNs.high >>> 0, true).toBigInt();
                        else if (typeof message.droppedRuntimeNs === "number")
                            object.droppedRuntimeNs = options.longs === $String ? $String(message.droppedRuntimeNs) : message.droppedRuntimeNs;
                        else
                            object.droppedRuntimeNs = options.longs === $String ? $util.Long.prototype.toString.call(message.droppedRuntimeNs) : options.longs === $Number ? new $util.LongBits(message.droppedRuntimeNs.low >>> 0, message.droppedRuntimeNs.high >>> 0).toNumber(true) : message.droppedRuntimeNs;
                    if (message.identityErrors != null && $Object.hasOwnProperty.call(message, "identityErrors"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.identityErrors = typeof message.identityErrors === "number" ? $BigInt(message.identityErrors) : $util.Long.fromBits(message.identityErrors.low >>> 0, message.identityErrors.high >>> 0, true).toBigInt();
                        else if (typeof message.identityErrors === "number")
                            object.identityErrors = options.longs === $String ? $String(message.identityErrors) : message.identityErrors;
                        else
                            object.identityErrors = options.longs === $String ? $util.Long.prototype.toString.call(message.identityErrors) : options.longs === $Number ? new $util.LongBits(message.identityErrors.low >>> 0, message.identityErrors.high >>> 0).toNumber(true) : message.identityErrors;
                    return object;
                };

                /**
                 * Converts this TaskLosses to JSON.
                 * @function toJSON
                 * @memberof doramcap.telemetry.v1.TaskLosses
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                TaskLosses.prototype.toJSON = function() {
                    return TaskLosses.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for TaskLosses
                 * @function getTypeUrl
                 * @memberof doramcap.telemetry.v1.TaskLosses
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                TaskLosses.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.telemetry.v1.TaskLosses";
                };

                return TaskLosses;
            })();

            v1.Snapshot = (function() {

                /**
                 * Properties of a Snapshot.
                 * @typedef {Object} doramcap.telemetry.v1.Snapshot.$Properties
                 * @property {number|null} [schemaVersion] Snapshot schemaVersion
                 * @property {number|Long|null} [sequence] Snapshot sequence
                 * @property {number|Long|null} [captureUnixNs] Snapshot captureUnixNs
                 * @property {number|Long|null} [monotonicStartNs] Snapshot monotonicStartNs
                 * @property {number|Long|null} [monotonicEndNs] Snapshot monotonicEndNs
                 * @property {number|Long|null} [droppedBatches] Snapshot droppedBatches
                 * @property {number|Long|null} [capacityOverflow] Snapshot capacityOverflow
                 * @property {Array.<doramcap.telemetry.v1.CpuRecord.$Properties>|null} [records] Snapshot records
                 * @property {Array.<doramcap.telemetry.v1.TaskRecord.$Properties>|null} [taskRecords] Snapshot taskRecords
                 * @property {doramcap.telemetry.v1.TaskLosses.$Properties|null} [taskLosses] Snapshot taskLosses
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Snapshot.
                 * @memberof doramcap.telemetry.v1
                 * @interface ISnapshot
                 * @augments doramcap.telemetry.v1.Snapshot.$Properties
                 * @deprecated Use doramcap.telemetry.v1.Snapshot.$Properties instead.
                 */

                /**
                 * Shape of a Snapshot.
                 * @typedef {doramcap.telemetry.v1.Snapshot.$Properties} doramcap.telemetry.v1.Snapshot.$Shape
                 */

                /**
                 * Constructs a new Snapshot.
                 * @memberof doramcap.telemetry.v1
                 * @classdesc Represents a Snapshot.
                 * @constructor
                 * @param {doramcap.telemetry.v1.Snapshot.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Snapshot = function (properties) {
                    this.records = [];
                    this.taskRecords = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Snapshot schemaVersion.
                 * @member {number} schemaVersion
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.schemaVersion = 0;

                /**
                 * Snapshot sequence.
                 * @member {number|Long} sequence
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.sequence = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot captureUnixNs.
                 * @member {number|Long} captureUnixNs
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.captureUnixNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot monotonicStartNs.
                 * @member {number|Long} monotonicStartNs
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.monotonicStartNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot monotonicEndNs.
                 * @member {number|Long} monotonicEndNs
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.monotonicEndNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot droppedBatches.
                 * @member {number|Long} droppedBatches
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.droppedBatches = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot capacityOverflow.
                 * @member {number|Long} capacityOverflow
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.capacityOverflow = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot records.
                 * @member {Array.<doramcap.telemetry.v1.CpuRecord.$Properties>} records
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.records = $util.emptyArray;

                /**
                 * Snapshot taskRecords.
                 * @member {Array.<doramcap.telemetry.v1.TaskRecord.$Properties>} taskRecords
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.taskRecords = $util.emptyArray;

                /**
                 * Snapshot taskLosses.
                 * @member {doramcap.telemetry.v1.TaskLosses.$Properties|null|undefined} taskLosses
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.taskLosses = null;

                /**
                 * Encodes the specified Snapshot message. Does not implicitly {@link doramcap.telemetry.v1.Snapshot.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @static
                 * @param {doramcap.telemetry.v1.Snapshot.$Properties} message Snapshot message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Snapshot.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.schemaVersion != null && $Object.hasOwnProperty.call(message, "schemaVersion") && message.schemaVersion !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.schemaVersion);
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence") && (typeof message.sequence === "object" ? message.sequence.low || message.sequence.high : message.sequence !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.sequence);
                    if (message.captureUnixNs != null && $Object.hasOwnProperty.call(message, "captureUnixNs") && (typeof message.captureUnixNs === "object" ? message.captureUnixNs.low || message.captureUnixNs.high : message.captureUnixNs !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.captureUnixNs);
                    if (message.monotonicStartNs != null && $Object.hasOwnProperty.call(message, "monotonicStartNs") && (typeof message.monotonicStartNs === "object" ? message.monotonicStartNs.low || message.monotonicStartNs.high : message.monotonicStartNs !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.monotonicStartNs);
                    if (message.monotonicEndNs != null && $Object.hasOwnProperty.call(message, "monotonicEndNs") && (typeof message.monotonicEndNs === "object" ? message.monotonicEndNs.low || message.monotonicEndNs.high : message.monotonicEndNs !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.monotonicEndNs);
                    if (message.droppedBatches != null && $Object.hasOwnProperty.call(message, "droppedBatches") && (typeof message.droppedBatches === "object" ? message.droppedBatches.low || message.droppedBatches.high : message.droppedBatches !== 0))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.droppedBatches);
                    if (message.capacityOverflow != null && $Object.hasOwnProperty.call(message, "capacityOverflow") && (typeof message.capacityOverflow === "object" ? message.capacityOverflow.low || message.capacityOverflow.high : message.capacityOverflow !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.capacityOverflow);
                    if (message.records != null && message.records.length)
                        for (let i = 0; i < message.records.length; ++i)
                            $root.doramcap.telemetry.v1.CpuRecord.encode(message.records[i], writer.uint32(/* id 8, wireType 2 =*/66).fork(), _depth + 1).ldelim();
                    if (message.taskRecords != null && message.taskRecords.length)
                        for (let i = 0; i < message.taskRecords.length; ++i)
                            $root.doramcap.telemetry.v1.TaskRecord.encode(message.taskRecords[i], writer.uint32(/* id 9, wireType 2 =*/74).fork(), _depth + 1).ldelim();
                    if (message.taskLosses != null && $Object.hasOwnProperty.call(message, "taskLosses"))
                        $root.doramcap.telemetry.v1.TaskLosses.encode(message.taskLosses, writer.uint32(/* id 10, wireType 2 =*/82).fork(), _depth + 1).ldelim();
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Snapshot message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.Snapshot & doramcap.telemetry.v1.Snapshot.$Shape} Snapshot
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Snapshot.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.telemetry.v1.Snapshot();
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
                                    message.schemaVersion = value;
                                else
                                    delete message.schemaVersion;
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
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.captureUnixNs = value;
                                else
                                    delete message.captureUnixNs;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.monotonicStartNs = value;
                                else
                                    delete message.monotonicStartNs;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.monotonicEndNs = value;
                                else
                                    delete message.monotonicEndNs;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.droppedBatches = value;
                                else
                                    delete message.droppedBatches;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.capacityOverflow = value;
                                else
                                    delete message.capacityOverflow;
                                continue;
                            }
                        case 8: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.records && message.records.length))
                                    message.records = [];
                                message.records.push($root.doramcap.telemetry.v1.CpuRecord.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 9: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.taskRecords && message.taskRecords.length))
                                    message.taskRecords = [];
                                message.taskRecords.push($root.doramcap.telemetry.v1.TaskRecord.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 10: {
                                if (wireType !== 2)
                                    break;
                                message.taskLosses = $root.doramcap.telemetry.v1.TaskLosses.decode(reader, reader.size(), $undefined, _depth + 1, message.taskLosses);
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
                 * Creates a Snapshot message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.telemetry.v1.Snapshot} Snapshot
                 */
                Snapshot.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.telemetry.v1.Snapshot)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.telemetry.v1.Snapshot: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.telemetry.v1.Snapshot();
                    if (object.schemaVersion != null)
                        if ($Number(object.schemaVersion) !== 0)
                            message.schemaVersion = object.schemaVersion >>> 0;
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
                    if (object.captureUnixNs != null)
                        if (typeof object.captureUnixNs === "object" ? object.captureUnixNs.low || object.captureUnixNs.high : $Number(object.captureUnixNs) !== 0)
                            if ($util.Long)
                                message.captureUnixNs = $util.Long.fromValue(object.captureUnixNs, true);
                            else if (typeof object.captureUnixNs === "string")
                                message.captureUnixNs = $parseInt(object.captureUnixNs, 10);
                            else if (typeof object.captureUnixNs === "number")
                                message.captureUnixNs = object.captureUnixNs;
                            else if (typeof object.captureUnixNs === "object")
                                message.captureUnixNs = new $util.LongBits(object.captureUnixNs.low >>> 0, object.captureUnixNs.high >>> 0).toNumber(true);
                    if (object.monotonicStartNs != null)
                        if (typeof object.monotonicStartNs === "object" ? object.monotonicStartNs.low || object.monotonicStartNs.high : $Number(object.monotonicStartNs) !== 0)
                            if ($util.Long)
                                message.monotonicStartNs = $util.Long.fromValue(object.monotonicStartNs, true);
                            else if (typeof object.monotonicStartNs === "string")
                                message.monotonicStartNs = $parseInt(object.monotonicStartNs, 10);
                            else if (typeof object.monotonicStartNs === "number")
                                message.monotonicStartNs = object.monotonicStartNs;
                            else if (typeof object.monotonicStartNs === "object")
                                message.monotonicStartNs = new $util.LongBits(object.monotonicStartNs.low >>> 0, object.monotonicStartNs.high >>> 0).toNumber(true);
                    if (object.monotonicEndNs != null)
                        if (typeof object.monotonicEndNs === "object" ? object.monotonicEndNs.low || object.monotonicEndNs.high : $Number(object.monotonicEndNs) !== 0)
                            if ($util.Long)
                                message.monotonicEndNs = $util.Long.fromValue(object.monotonicEndNs, true);
                            else if (typeof object.monotonicEndNs === "string")
                                message.monotonicEndNs = $parseInt(object.monotonicEndNs, 10);
                            else if (typeof object.monotonicEndNs === "number")
                                message.monotonicEndNs = object.monotonicEndNs;
                            else if (typeof object.monotonicEndNs === "object")
                                message.monotonicEndNs = new $util.LongBits(object.monotonicEndNs.low >>> 0, object.monotonicEndNs.high >>> 0).toNumber(true);
                    if (object.droppedBatches != null)
                        if (typeof object.droppedBatches === "object" ? object.droppedBatches.low || object.droppedBatches.high : $Number(object.droppedBatches) !== 0)
                            if ($util.Long)
                                message.droppedBatches = $util.Long.fromValue(object.droppedBatches, true);
                            else if (typeof object.droppedBatches === "string")
                                message.droppedBatches = $parseInt(object.droppedBatches, 10);
                            else if (typeof object.droppedBatches === "number")
                                message.droppedBatches = object.droppedBatches;
                            else if (typeof object.droppedBatches === "object")
                                message.droppedBatches = new $util.LongBits(object.droppedBatches.low >>> 0, object.droppedBatches.high >>> 0).toNumber(true);
                    if (object.capacityOverflow != null)
                        if (typeof object.capacityOverflow === "object" ? object.capacityOverflow.low || object.capacityOverflow.high : $Number(object.capacityOverflow) !== 0)
                            if ($util.Long)
                                message.capacityOverflow = $util.Long.fromValue(object.capacityOverflow, true);
                            else if (typeof object.capacityOverflow === "string")
                                message.capacityOverflow = $parseInt(object.capacityOverflow, 10);
                            else if (typeof object.capacityOverflow === "number")
                                message.capacityOverflow = object.capacityOverflow;
                            else if (typeof object.capacityOverflow === "object")
                                message.capacityOverflow = new $util.LongBits(object.capacityOverflow.low >>> 0, object.capacityOverflow.high >>> 0).toNumber(true);
                    if (object.records) {
                        if (!$Array.isArray(object.records))
                            throw $TypeError(".doramcap.telemetry.v1.Snapshot.records: array expected");
                        message.records = $Array(object.records.length);
                        for (let i = 0; i < object.records.length; ++i) {
                            if (!$util.isObject(object.records[i]))
                                throw $TypeError(".doramcap.telemetry.v1.Snapshot.records: object expected");
                            message.records[i] = $root.doramcap.telemetry.v1.CpuRecord.fromObject(object.records[i], _depth + 1);
                        }
                    }
                    if (object.taskRecords) {
                        if (!$Array.isArray(object.taskRecords))
                            throw $TypeError(".doramcap.telemetry.v1.Snapshot.taskRecords: array expected");
                        message.taskRecords = $Array(object.taskRecords.length);
                        for (let i = 0; i < object.taskRecords.length; ++i) {
                            if (!$util.isObject(object.taskRecords[i]))
                                throw $TypeError(".doramcap.telemetry.v1.Snapshot.taskRecords: object expected");
                            message.taskRecords[i] = $root.doramcap.telemetry.v1.TaskRecord.fromObject(object.taskRecords[i], _depth + 1);
                        }
                    }
                    if (object.taskLosses != null) {
                        if (!$util.isObject(object.taskLosses))
                            throw $TypeError(".doramcap.telemetry.v1.Snapshot.taskLosses: object expected");
                        message.taskLosses = $root.doramcap.telemetry.v1.TaskLosses.fromObject(object.taskLosses, _depth + 1);
                    }
                    return message;
                };

                /**
                 * Creates a plain object from a Snapshot message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @static
                 * @param {doramcap.telemetry.v1.Snapshot} message Snapshot
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Snapshot.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.arrays || options.defaults) {
                        object.records = [];
                        object.taskRecords = [];
                    }
                    if (options.defaults) {
                        object.schemaVersion = 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.sequence = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.sequence = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.captureUnixNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.captureUnixNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.monotonicStartNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.monotonicStartNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.monotonicEndNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.monotonicEndNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.droppedBatches = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.droppedBatches = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.capacityOverflow = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.capacityOverflow = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.taskLosses = null;
                    }
                    if (message.schemaVersion != null && $Object.hasOwnProperty.call(message, "schemaVersion"))
                        object.schemaVersion = message.schemaVersion;
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.sequence = typeof message.sequence === "number" ? $BigInt(message.sequence) : $util.Long.fromBits(message.sequence.low >>> 0, message.sequence.high >>> 0, true).toBigInt();
                        else if (typeof message.sequence === "number")
                            object.sequence = options.longs === $String ? $String(message.sequence) : message.sequence;
                        else
                            object.sequence = options.longs === $String ? $util.Long.prototype.toString.call(message.sequence) : options.longs === $Number ? new $util.LongBits(message.sequence.low >>> 0, message.sequence.high >>> 0).toNumber(true) : message.sequence;
                    if (message.captureUnixNs != null && $Object.hasOwnProperty.call(message, "captureUnixNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.captureUnixNs = typeof message.captureUnixNs === "number" ? $BigInt(message.captureUnixNs) : $util.Long.fromBits(message.captureUnixNs.low >>> 0, message.captureUnixNs.high >>> 0, true).toBigInt();
                        else if (typeof message.captureUnixNs === "number")
                            object.captureUnixNs = options.longs === $String ? $String(message.captureUnixNs) : message.captureUnixNs;
                        else
                            object.captureUnixNs = options.longs === $String ? $util.Long.prototype.toString.call(message.captureUnixNs) : options.longs === $Number ? new $util.LongBits(message.captureUnixNs.low >>> 0, message.captureUnixNs.high >>> 0).toNumber(true) : message.captureUnixNs;
                    if (message.monotonicStartNs != null && $Object.hasOwnProperty.call(message, "monotonicStartNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.monotonicStartNs = typeof message.monotonicStartNs === "number" ? $BigInt(message.monotonicStartNs) : $util.Long.fromBits(message.monotonicStartNs.low >>> 0, message.monotonicStartNs.high >>> 0, true).toBigInt();
                        else if (typeof message.monotonicStartNs === "number")
                            object.monotonicStartNs = options.longs === $String ? $String(message.monotonicStartNs) : message.monotonicStartNs;
                        else
                            object.monotonicStartNs = options.longs === $String ? $util.Long.prototype.toString.call(message.monotonicStartNs) : options.longs === $Number ? new $util.LongBits(message.monotonicStartNs.low >>> 0, message.monotonicStartNs.high >>> 0).toNumber(true) : message.monotonicStartNs;
                    if (message.monotonicEndNs != null && $Object.hasOwnProperty.call(message, "monotonicEndNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.monotonicEndNs = typeof message.monotonicEndNs === "number" ? $BigInt(message.monotonicEndNs) : $util.Long.fromBits(message.monotonicEndNs.low >>> 0, message.monotonicEndNs.high >>> 0, true).toBigInt();
                        else if (typeof message.monotonicEndNs === "number")
                            object.monotonicEndNs = options.longs === $String ? $String(message.monotonicEndNs) : message.monotonicEndNs;
                        else
                            object.monotonicEndNs = options.longs === $String ? $util.Long.prototype.toString.call(message.monotonicEndNs) : options.longs === $Number ? new $util.LongBits(message.monotonicEndNs.low >>> 0, message.monotonicEndNs.high >>> 0).toNumber(true) : message.monotonicEndNs;
                    if (message.droppedBatches != null && $Object.hasOwnProperty.call(message, "droppedBatches"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.droppedBatches = typeof message.droppedBatches === "number" ? $BigInt(message.droppedBatches) : $util.Long.fromBits(message.droppedBatches.low >>> 0, message.droppedBatches.high >>> 0, true).toBigInt();
                        else if (typeof message.droppedBatches === "number")
                            object.droppedBatches = options.longs === $String ? $String(message.droppedBatches) : message.droppedBatches;
                        else
                            object.droppedBatches = options.longs === $String ? $util.Long.prototype.toString.call(message.droppedBatches) : options.longs === $Number ? new $util.LongBits(message.droppedBatches.low >>> 0, message.droppedBatches.high >>> 0).toNumber(true) : message.droppedBatches;
                    if (message.capacityOverflow != null && $Object.hasOwnProperty.call(message, "capacityOverflow"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.capacityOverflow = typeof message.capacityOverflow === "number" ? $BigInt(message.capacityOverflow) : $util.Long.fromBits(message.capacityOverflow.low >>> 0, message.capacityOverflow.high >>> 0, true).toBigInt();
                        else if (typeof message.capacityOverflow === "number")
                            object.capacityOverflow = options.longs === $String ? $String(message.capacityOverflow) : message.capacityOverflow;
                        else
                            object.capacityOverflow = options.longs === $String ? $util.Long.prototype.toString.call(message.capacityOverflow) : options.longs === $Number ? new $util.LongBits(message.capacityOverflow.low >>> 0, message.capacityOverflow.high >>> 0).toNumber(true) : message.capacityOverflow;
                    if (message.records && message.records.length) {
                        object.records = $Array(message.records.length);
                        for (let j = 0; j < message.records.length; ++j)
                            object.records[j] = $root.doramcap.telemetry.v1.CpuRecord.toObject(message.records[j], options, _depth + 1);
                    }
                    if (message.taskRecords && message.taskRecords.length) {
                        object.taskRecords = $Array(message.taskRecords.length);
                        for (let j = 0; j < message.taskRecords.length; ++j)
                            object.taskRecords[j] = $root.doramcap.telemetry.v1.TaskRecord.toObject(message.taskRecords[j], options, _depth + 1);
                    }
                    if (message.taskLosses != null && $Object.hasOwnProperty.call(message, "taskLosses"))
                        object.taskLosses = $root.doramcap.telemetry.v1.TaskLosses.toObject(message.taskLosses, options, _depth + 1);
                    return object;
                };

                /**
                 * Converts this Snapshot to JSON.
                 * @function toJSON
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Snapshot.prototype.toJSON = function() {
                    return Snapshot.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Snapshot
                 * @function getTypeUrl
                 * @memberof doramcap.telemetry.v1.Snapshot
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Snapshot.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.telemetry.v1.Snapshot";
                };

                return Snapshot;
            })();

            v1.DictionaryEntry = (function() {

                /**
                 * Properties of a DictionaryEntry.
                 * @typedef {Object} doramcap.telemetry.v1.DictionaryEntry.$Properties
                 * @property {number|Long|null} [id] DictionaryEntry id
                 * @property {number|Long|null} [parentId] DictionaryEntry parentId
                 * @property {string|null} [path] DictionaryEntry path
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a DictionaryEntry.
                 * @memberof doramcap.telemetry.v1
                 * @interface IDictionaryEntry
                 * @augments doramcap.telemetry.v1.DictionaryEntry.$Properties
                 * @deprecated Use doramcap.telemetry.v1.DictionaryEntry.$Properties instead.
                 */

                /**
                 * Shape of a DictionaryEntry.
                 * @typedef {doramcap.telemetry.v1.DictionaryEntry.$Properties} doramcap.telemetry.v1.DictionaryEntry.$Shape
                 */

                /**
                 * Constructs a new DictionaryEntry.
                 * @memberof doramcap.telemetry.v1
                 * @classdesc Represents a DictionaryEntry.
                 * @constructor
                 * @param {doramcap.telemetry.v1.DictionaryEntry.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const DictionaryEntry = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * DictionaryEntry id.
                 * @member {number|Long} id
                 * @memberof doramcap.telemetry.v1.DictionaryEntry
                 * @instance
                 */
                DictionaryEntry.prototype.id = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * DictionaryEntry parentId.
                 * @member {number|Long} parentId
                 * @memberof doramcap.telemetry.v1.DictionaryEntry
                 * @instance
                 */
                DictionaryEntry.prototype.parentId = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * DictionaryEntry path.
                 * @member {string|null|undefined} path
                 * @memberof doramcap.telemetry.v1.DictionaryEntry
                 * @instance
                 */
                DictionaryEntry.prototype.path = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(DictionaryEntry.prototype, "_path", {
                    get: $util.oneOfGetter($oneOfFields = ["path"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified DictionaryEntry message. Does not implicitly {@link doramcap.telemetry.v1.DictionaryEntry.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.telemetry.v1.DictionaryEntry
                 * @static
                 * @param {doramcap.telemetry.v1.DictionaryEntry.$Properties} message DictionaryEntry message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                DictionaryEntry.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.id != null && $Object.hasOwnProperty.call(message, "id") && (typeof message.id === "object" ? message.id.low || message.id.high : message.id !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.id);
                    if (message.parentId != null && $Object.hasOwnProperty.call(message, "parentId") && (typeof message.parentId === "object" ? message.parentId.low || message.parentId.high : message.parentId !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.parentId);
                    if (message.path != null && $Object.hasOwnProperty.call(message, "path"))
                        writer.uint32(/* id 3, wireType 2 =*/26).string(message.path);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a DictionaryEntry message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.telemetry.v1.DictionaryEntry
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.DictionaryEntry & doramcap.telemetry.v1.DictionaryEntry.$Shape} DictionaryEntry
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                DictionaryEntry.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.telemetry.v1.DictionaryEntry();
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
                                    message.id = value;
                                else
                                    delete message.id;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.parentId = value;
                                else
                                    delete message.parentId;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                message.path = reader.stringVerify();
                                message._path = "path";
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
                 * Creates a DictionaryEntry message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.telemetry.v1.DictionaryEntry
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.telemetry.v1.DictionaryEntry} DictionaryEntry
                 */
                DictionaryEntry.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.telemetry.v1.DictionaryEntry)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.telemetry.v1.DictionaryEntry: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.telemetry.v1.DictionaryEntry();
                    if (object.id != null)
                        if (typeof object.id === "object" ? object.id.low || object.id.high : $Number(object.id) !== 0)
                            if ($util.Long)
                                message.id = $util.Long.fromValue(object.id, true);
                            else if (typeof object.id === "string")
                                message.id = $parseInt(object.id, 10);
                            else if (typeof object.id === "number")
                                message.id = object.id;
                            else if (typeof object.id === "object")
                                message.id = new $util.LongBits(object.id.low >>> 0, object.id.high >>> 0).toNumber(true);
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
                    if (object.path != null)
                        message.path = $String(object.path);
                    return message;
                };

                /**
                 * Creates a plain object from a DictionaryEntry message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.telemetry.v1.DictionaryEntry
                 * @static
                 * @param {doramcap.telemetry.v1.DictionaryEntry} message DictionaryEntry
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                DictionaryEntry.toObject = function (message, options, _depth) {
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
                            object.id = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.id = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.parentId = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.parentId = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.id != null && $Object.hasOwnProperty.call(message, "id"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.id = typeof message.id === "number" ? $BigInt(message.id) : $util.Long.fromBits(message.id.low >>> 0, message.id.high >>> 0, true).toBigInt();
                        else if (typeof message.id === "number")
                            object.id = options.longs === $String ? $String(message.id) : message.id;
                        else
                            object.id = options.longs === $String ? $util.Long.prototype.toString.call(message.id) : options.longs === $Number ? new $util.LongBits(message.id.low >>> 0, message.id.high >>> 0).toNumber(true) : message.id;
                    if (message.parentId != null && $Object.hasOwnProperty.call(message, "parentId"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.parentId = typeof message.parentId === "number" ? $BigInt(message.parentId) : $util.Long.fromBits(message.parentId.low >>> 0, message.parentId.high >>> 0, true).toBigInt();
                        else if (typeof message.parentId === "number")
                            object.parentId = options.longs === $String ? $String(message.parentId) : message.parentId;
                        else
                            object.parentId = options.longs === $String ? $util.Long.prototype.toString.call(message.parentId) : options.longs === $Number ? new $util.LongBits(message.parentId.low >>> 0, message.parentId.high >>> 0).toNumber(true) : message.parentId;
                    if (message.path != null && $Object.hasOwnProperty.call(message, "path"))
                        object.path = message.path;
                    return object;
                };

                /**
                 * Converts this DictionaryEntry to JSON.
                 * @function toJSON
                 * @memberof doramcap.telemetry.v1.DictionaryEntry
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                DictionaryEntry.prototype.toJSON = function() {
                    return DictionaryEntry.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for DictionaryEntry
                 * @function getTypeUrl
                 * @memberof doramcap.telemetry.v1.DictionaryEntry
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                DictionaryEntry.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.telemetry.v1.DictionaryEntry";
                };

                return DictionaryEntry;
            })();

            v1.Ingress = (function() {

                /**
                 * Properties of an Ingress.
                 * @typedef {Object} doramcap.telemetry.v1.Ingress.$Properties
                 * @property {number|Long|null} [totalBytes] Ingress totalBytes
                 * @property {number|Long|null} [totalMessages] Ingress totalMessages
                 * @property {number|null} [bytesPerSecond] Ingress bytesPerSecond
                 * @property {number|null} [messagesPerSecond] Ingress messagesPerSecond
                 * @property {number|null} [captureToReceiveMs] Ingress captureToReceiveMs
                 * @property {number|null} [meanCaptureToReceiveMs] Ingress meanCaptureToReceiveMs
                 * @property {number|Long|null} [invalidTimestamps] Ingress invalidTimestamps
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of an Ingress.
                 * @memberof doramcap.telemetry.v1
                 * @interface IIngress
                 * @augments doramcap.telemetry.v1.Ingress.$Properties
                 * @deprecated Use doramcap.telemetry.v1.Ingress.$Properties instead.
                 */

                /**
                 * Shape of an Ingress.
                 * @typedef {doramcap.telemetry.v1.Ingress.$Properties} doramcap.telemetry.v1.Ingress.$Shape
                 */

                /**
                 * Constructs a new Ingress.
                 * @memberof doramcap.telemetry.v1
                 * @classdesc Represents an Ingress.
                 * @constructor
                 * @param {doramcap.telemetry.v1.Ingress.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Ingress = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Ingress totalBytes.
                 * @member {number|Long} totalBytes
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @instance
                 */
                Ingress.prototype.totalBytes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ingress totalMessages.
                 * @member {number|Long} totalMessages
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @instance
                 */
                Ingress.prototype.totalMessages = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ingress bytesPerSecond.
                 * @member {number|null|undefined} bytesPerSecond
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @instance
                 */
                Ingress.prototype.bytesPerSecond = null;

                /**
                 * Ingress messagesPerSecond.
                 * @member {number|null|undefined} messagesPerSecond
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @instance
                 */
                Ingress.prototype.messagesPerSecond = null;

                /**
                 * Ingress captureToReceiveMs.
                 * @member {number|null|undefined} captureToReceiveMs
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @instance
                 */
                Ingress.prototype.captureToReceiveMs = null;

                /**
                 * Ingress meanCaptureToReceiveMs.
                 * @member {number|null|undefined} meanCaptureToReceiveMs
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @instance
                 */
                Ingress.prototype.meanCaptureToReceiveMs = null;

                /**
                 * Ingress invalidTimestamps.
                 * @member {number|Long} invalidTimestamps
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @instance
                 */
                Ingress.prototype.invalidTimestamps = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Ingress.prototype, "_bytesPerSecond", {
                    get: $util.oneOfGetter($oneOfFields = ["bytesPerSecond"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Ingress.prototype, "_messagesPerSecond", {
                    get: $util.oneOfGetter($oneOfFields = ["messagesPerSecond"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Ingress.prototype, "_captureToReceiveMs", {
                    get: $util.oneOfGetter($oneOfFields = ["captureToReceiveMs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Ingress.prototype, "_meanCaptureToReceiveMs", {
                    get: $util.oneOfGetter($oneOfFields = ["meanCaptureToReceiveMs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Ingress message. Does not implicitly {@link doramcap.telemetry.v1.Ingress.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @static
                 * @param {doramcap.telemetry.v1.Ingress.$Properties} message Ingress message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Ingress.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.totalBytes != null && $Object.hasOwnProperty.call(message, "totalBytes") && (typeof message.totalBytes === "object" ? message.totalBytes.low || message.totalBytes.high : message.totalBytes !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.totalBytes);
                    if (message.totalMessages != null && $Object.hasOwnProperty.call(message, "totalMessages") && (typeof message.totalMessages === "object" ? message.totalMessages.low || message.totalMessages.high : message.totalMessages !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.totalMessages);
                    if (message.bytesPerSecond != null && $Object.hasOwnProperty.call(message, "bytesPerSecond"))
                        writer.uint32(/* id 3, wireType 1 =*/25).double(message.bytesPerSecond);
                    if (message.messagesPerSecond != null && $Object.hasOwnProperty.call(message, "messagesPerSecond"))
                        writer.uint32(/* id 4, wireType 1 =*/33).double(message.messagesPerSecond);
                    if (message.captureToReceiveMs != null && $Object.hasOwnProperty.call(message, "captureToReceiveMs"))
                        writer.uint32(/* id 5, wireType 1 =*/41).double(message.captureToReceiveMs);
                    if (message.meanCaptureToReceiveMs != null && $Object.hasOwnProperty.call(message, "meanCaptureToReceiveMs"))
                        writer.uint32(/* id 6, wireType 1 =*/49).double(message.meanCaptureToReceiveMs);
                    if (message.invalidTimestamps != null && $Object.hasOwnProperty.call(message, "invalidTimestamps") && (typeof message.invalidTimestamps === "object" ? message.invalidTimestamps.low || message.invalidTimestamps.high : message.invalidTimestamps !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.invalidTimestamps);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes an Ingress message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.Ingress & doramcap.telemetry.v1.Ingress.$Shape} Ingress
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Ingress.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.telemetry.v1.Ingress();
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
                                    message.totalBytes = value;
                                else
                                    delete message.totalBytes;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.totalMessages = value;
                                else
                                    delete message.totalMessages;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 1)
                                    break;
                                message.bytesPerSecond = reader.double();
                                message._bytesPerSecond = "bytesPerSecond";
                                continue;
                            }
                        case 4: {
                                if (wireType !== 1)
                                    break;
                                message.messagesPerSecond = reader.double();
                                message._messagesPerSecond = "messagesPerSecond";
                                continue;
                            }
                        case 5: {
                                if (wireType !== 1)
                                    break;
                                message.captureToReceiveMs = reader.double();
                                message._captureToReceiveMs = "captureToReceiveMs";
                                continue;
                            }
                        case 6: {
                                if (wireType !== 1)
                                    break;
                                message.meanCaptureToReceiveMs = reader.double();
                                message._meanCaptureToReceiveMs = "meanCaptureToReceiveMs";
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.invalidTimestamps = value;
                                else
                                    delete message.invalidTimestamps;
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
                 * Creates an Ingress message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.telemetry.v1.Ingress} Ingress
                 */
                Ingress.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.telemetry.v1.Ingress)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.telemetry.v1.Ingress: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.telemetry.v1.Ingress();
                    if (object.totalBytes != null)
                        if (typeof object.totalBytes === "object" ? object.totalBytes.low || object.totalBytes.high : $Number(object.totalBytes) !== 0)
                            if ($util.Long)
                                message.totalBytes = $util.Long.fromValue(object.totalBytes, true);
                            else if (typeof object.totalBytes === "string")
                                message.totalBytes = $parseInt(object.totalBytes, 10);
                            else if (typeof object.totalBytes === "number")
                                message.totalBytes = object.totalBytes;
                            else if (typeof object.totalBytes === "object")
                                message.totalBytes = new $util.LongBits(object.totalBytes.low >>> 0, object.totalBytes.high >>> 0).toNumber(true);
                    if (object.totalMessages != null)
                        if (typeof object.totalMessages === "object" ? object.totalMessages.low || object.totalMessages.high : $Number(object.totalMessages) !== 0)
                            if ($util.Long)
                                message.totalMessages = $util.Long.fromValue(object.totalMessages, true);
                            else if (typeof object.totalMessages === "string")
                                message.totalMessages = $parseInt(object.totalMessages, 10);
                            else if (typeof object.totalMessages === "number")
                                message.totalMessages = object.totalMessages;
                            else if (typeof object.totalMessages === "object")
                                message.totalMessages = new $util.LongBits(object.totalMessages.low >>> 0, object.totalMessages.high >>> 0).toNumber(true);
                    if (object.bytesPerSecond != null)
                        message.bytesPerSecond = $Number(object.bytesPerSecond);
                    if (object.messagesPerSecond != null)
                        message.messagesPerSecond = $Number(object.messagesPerSecond);
                    if (object.captureToReceiveMs != null)
                        message.captureToReceiveMs = $Number(object.captureToReceiveMs);
                    if (object.meanCaptureToReceiveMs != null)
                        message.meanCaptureToReceiveMs = $Number(object.meanCaptureToReceiveMs);
                    if (object.invalidTimestamps != null)
                        if (typeof object.invalidTimestamps === "object" ? object.invalidTimestamps.low || object.invalidTimestamps.high : $Number(object.invalidTimestamps) !== 0)
                            if ($util.Long)
                                message.invalidTimestamps = $util.Long.fromValue(object.invalidTimestamps, true);
                            else if (typeof object.invalidTimestamps === "string")
                                message.invalidTimestamps = $parseInt(object.invalidTimestamps, 10);
                            else if (typeof object.invalidTimestamps === "number")
                                message.invalidTimestamps = object.invalidTimestamps;
                            else if (typeof object.invalidTimestamps === "object")
                                message.invalidTimestamps = new $util.LongBits(object.invalidTimestamps.low >>> 0, object.invalidTimestamps.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from an Ingress message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @static
                 * @param {doramcap.telemetry.v1.Ingress} message Ingress
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Ingress.toObject = function (message, options, _depth) {
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
                            object.totalBytes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.totalBytes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.totalMessages = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.totalMessages = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.invalidTimestamps = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.invalidTimestamps = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.totalBytes != null && $Object.hasOwnProperty.call(message, "totalBytes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.totalBytes = typeof message.totalBytes === "number" ? $BigInt(message.totalBytes) : $util.Long.fromBits(message.totalBytes.low >>> 0, message.totalBytes.high >>> 0, true).toBigInt();
                        else if (typeof message.totalBytes === "number")
                            object.totalBytes = options.longs === $String ? $String(message.totalBytes) : message.totalBytes;
                        else
                            object.totalBytes = options.longs === $String ? $util.Long.prototype.toString.call(message.totalBytes) : options.longs === $Number ? new $util.LongBits(message.totalBytes.low >>> 0, message.totalBytes.high >>> 0).toNumber(true) : message.totalBytes;
                    if (message.totalMessages != null && $Object.hasOwnProperty.call(message, "totalMessages"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.totalMessages = typeof message.totalMessages === "number" ? $BigInt(message.totalMessages) : $util.Long.fromBits(message.totalMessages.low >>> 0, message.totalMessages.high >>> 0, true).toBigInt();
                        else if (typeof message.totalMessages === "number")
                            object.totalMessages = options.longs === $String ? $String(message.totalMessages) : message.totalMessages;
                        else
                            object.totalMessages = options.longs === $String ? $util.Long.prototype.toString.call(message.totalMessages) : options.longs === $Number ? new $util.LongBits(message.totalMessages.low >>> 0, message.totalMessages.high >>> 0).toNumber(true) : message.totalMessages;
                    if (message.bytesPerSecond != null && $Object.hasOwnProperty.call(message, "bytesPerSecond"))
                        object.bytesPerSecond = options.json && !$isFinite(message.bytesPerSecond) ? $String(message.bytesPerSecond) : message.bytesPerSecond;
                    if (message.messagesPerSecond != null && $Object.hasOwnProperty.call(message, "messagesPerSecond"))
                        object.messagesPerSecond = options.json && !$isFinite(message.messagesPerSecond) ? $String(message.messagesPerSecond) : message.messagesPerSecond;
                    if (message.captureToReceiveMs != null && $Object.hasOwnProperty.call(message, "captureToReceiveMs"))
                        object.captureToReceiveMs = options.json && !$isFinite(message.captureToReceiveMs) ? $String(message.captureToReceiveMs) : message.captureToReceiveMs;
                    if (message.meanCaptureToReceiveMs != null && $Object.hasOwnProperty.call(message, "meanCaptureToReceiveMs"))
                        object.meanCaptureToReceiveMs = options.json && !$isFinite(message.meanCaptureToReceiveMs) ? $String(message.meanCaptureToReceiveMs) : message.meanCaptureToReceiveMs;
                    if (message.invalidTimestamps != null && $Object.hasOwnProperty.call(message, "invalidTimestamps"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.invalidTimestamps = typeof message.invalidTimestamps === "number" ? $BigInt(message.invalidTimestamps) : $util.Long.fromBits(message.invalidTimestamps.low >>> 0, message.invalidTimestamps.high >>> 0, true).toBigInt();
                        else if (typeof message.invalidTimestamps === "number")
                            object.invalidTimestamps = options.longs === $String ? $String(message.invalidTimestamps) : message.invalidTimestamps;
                        else
                            object.invalidTimestamps = options.longs === $String ? $util.Long.prototype.toString.call(message.invalidTimestamps) : options.longs === $Number ? new $util.LongBits(message.invalidTimestamps.low >>> 0, message.invalidTimestamps.high >>> 0).toNumber(true) : message.invalidTimestamps;
                    return object;
                };

                /**
                 * Converts this Ingress to JSON.
                 * @function toJSON
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Ingress.prototype.toJSON = function() {
                    return Ingress.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Ingress
                 * @function getTypeUrl
                 * @memberof doramcap.telemetry.v1.Ingress
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Ingress.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.telemetry.v1.Ingress";
                };

                return Ingress;
            })();

            v1.SnapshotFrame = (function() {

                /**
                 * Properties of a SnapshotFrame.
                 * @typedef {Object} doramcap.telemetry.v1.SnapshotFrame.$Properties
                 * @property {number|null} [version] SnapshotFrame version
                 * @property {doramcap.telemetry.v1.Snapshot.$Properties|null} [snapshot] SnapshotFrame snapshot
                 * @property {string|null} [source] SnapshotFrame source
                 * @property {boolean|null} [taskCollectionEnabled] SnapshotFrame taskCollectionEnabled
                 * @property {number|Long|null} [taskCapacity] SnapshotFrame taskCapacity
                 * @property {string|null} [sessionId] SnapshotFrame sessionId
                 * @property {number|Long|null} [dictionaryRevision] SnapshotFrame dictionaryRevision
                 * @property {boolean|null} [dictionaryFull] SnapshotFrame dictionaryFull
                 * @property {number|Long|null} [dictionaryOverflow] SnapshotFrame dictionaryOverflow
                 * @property {Array.<doramcap.telemetry.v1.DictionaryEntry.$Properties>|null} [dictionary] SnapshotFrame dictionary
                 * @property {doramcap.telemetry.v1.Ingress.$Properties|null} [viewerIngress] SnapshotFrame viewerIngress
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a SnapshotFrame.
                 * @memberof doramcap.telemetry.v1
                 * @interface ISnapshotFrame
                 * @augments doramcap.telemetry.v1.SnapshotFrame.$Properties
                 * @deprecated Use doramcap.telemetry.v1.SnapshotFrame.$Properties instead.
                 */

                /**
                 * Shape of a SnapshotFrame.
                 * @typedef {doramcap.telemetry.v1.SnapshotFrame.$Properties} doramcap.telemetry.v1.SnapshotFrame.$Shape
                 */

                /**
                 * Constructs a new SnapshotFrame.
                 * @memberof doramcap.telemetry.v1
                 * @classdesc Represents a SnapshotFrame.
                 * @constructor
                 * @param {doramcap.telemetry.v1.SnapshotFrame.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const SnapshotFrame = function (properties) {
                    this.dictionary = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * SnapshotFrame version.
                 * @member {number} version
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.version = 0;

                /**
                 * SnapshotFrame snapshot.
                 * @member {doramcap.telemetry.v1.Snapshot.$Properties|null|undefined} snapshot
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.snapshot = null;

                /**
                 * SnapshotFrame source.
                 * @member {string} source
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.source = "";

                /**
                 * SnapshotFrame taskCollectionEnabled.
                 * @member {boolean} taskCollectionEnabled
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.taskCollectionEnabled = false;

                /**
                 * SnapshotFrame taskCapacity.
                 * @member {number|Long} taskCapacity
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.taskCapacity = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * SnapshotFrame sessionId.
                 * @member {string} sessionId
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.sessionId = "";

                /**
                 * SnapshotFrame dictionaryRevision.
                 * @member {number|Long} dictionaryRevision
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.dictionaryRevision = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * SnapshotFrame dictionaryFull.
                 * @member {boolean} dictionaryFull
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.dictionaryFull = false;

                /**
                 * SnapshotFrame dictionaryOverflow.
                 * @member {number|Long} dictionaryOverflow
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.dictionaryOverflow = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * SnapshotFrame dictionary.
                 * @member {Array.<doramcap.telemetry.v1.DictionaryEntry.$Properties>} dictionary
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.dictionary = $util.emptyArray;

                /**
                 * SnapshotFrame viewerIngress.
                 * @member {doramcap.telemetry.v1.Ingress.$Properties|null|undefined} viewerIngress
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 */
                SnapshotFrame.prototype.viewerIngress = null;

                /**
                 * Encodes the specified SnapshotFrame message. Does not implicitly {@link doramcap.telemetry.v1.SnapshotFrame.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @static
                 * @param {doramcap.telemetry.v1.SnapshotFrame.$Properties} message SnapshotFrame message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                SnapshotFrame.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version") && message.version !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.version);
                    if (message.snapshot != null && $Object.hasOwnProperty.call(message, "snapshot"))
                        $root.doramcap.telemetry.v1.Snapshot.encode(message.snapshot, writer.uint32(/* id 2, wireType 2 =*/18).fork(), _depth + 1).ldelim();
                    if (message.source != null && $Object.hasOwnProperty.call(message, "source") && message.source !== "")
                        writer.uint32(/* id 3, wireType 2 =*/26).string(message.source);
                    if (message.taskCollectionEnabled != null && $Object.hasOwnProperty.call(message, "taskCollectionEnabled") && message.taskCollectionEnabled !== false)
                        writer.uint32(/* id 4, wireType 0 =*/32).bool(message.taskCollectionEnabled);
                    if (message.taskCapacity != null && $Object.hasOwnProperty.call(message, "taskCapacity") && (typeof message.taskCapacity === "object" ? message.taskCapacity.low || message.taskCapacity.high : message.taskCapacity !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.taskCapacity);
                    if (message.sessionId != null && $Object.hasOwnProperty.call(message, "sessionId") && message.sessionId !== "")
                        writer.uint32(/* id 6, wireType 2 =*/50).string(message.sessionId);
                    if (message.dictionaryRevision != null && $Object.hasOwnProperty.call(message, "dictionaryRevision") && (typeof message.dictionaryRevision === "object" ? message.dictionaryRevision.low || message.dictionaryRevision.high : message.dictionaryRevision !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.dictionaryRevision);
                    if (message.dictionaryFull != null && $Object.hasOwnProperty.call(message, "dictionaryFull") && message.dictionaryFull !== false)
                        writer.uint32(/* id 8, wireType 0 =*/64).bool(message.dictionaryFull);
                    if (message.dictionaryOverflow != null && $Object.hasOwnProperty.call(message, "dictionaryOverflow") && (typeof message.dictionaryOverflow === "object" ? message.dictionaryOverflow.low || message.dictionaryOverflow.high : message.dictionaryOverflow !== 0))
                        writer.uint32(/* id 9, wireType 0 =*/72).uint64(message.dictionaryOverflow);
                    if (message.dictionary != null && message.dictionary.length)
                        for (let i = 0; i < message.dictionary.length; ++i)
                            $root.doramcap.telemetry.v1.DictionaryEntry.encode(message.dictionary[i], writer.uint32(/* id 10, wireType 2 =*/82).fork(), _depth + 1).ldelim();
                    if (message.viewerIngress != null && $Object.hasOwnProperty.call(message, "viewerIngress"))
                        $root.doramcap.telemetry.v1.Ingress.encode(message.viewerIngress, writer.uint32(/* id 11, wireType 2 =*/90).fork(), _depth + 1).ldelim();
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a SnapshotFrame message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.SnapshotFrame & doramcap.telemetry.v1.SnapshotFrame.$Shape} SnapshotFrame
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                SnapshotFrame.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.telemetry.v1.SnapshotFrame();
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
                                message.snapshot = $root.doramcap.telemetry.v1.Snapshot.decode(reader, reader.size(), $undefined, _depth + 1, message.snapshot);
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.source = value;
                                else
                                    delete message.source;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.bool())
                                    message.taskCollectionEnabled = value;
                                else
                                    delete message.taskCollectionEnabled;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.taskCapacity = value;
                                else
                                    delete message.taskCapacity;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.sessionId = value;
                                else
                                    delete message.sessionId;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.dictionaryRevision = value;
                                else
                                    delete message.dictionaryRevision;
                                continue;
                            }
                        case 8: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.bool())
                                    message.dictionaryFull = value;
                                else
                                    delete message.dictionaryFull;
                                continue;
                            }
                        case 9: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.dictionaryOverflow = value;
                                else
                                    delete message.dictionaryOverflow;
                                continue;
                            }
                        case 10: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.dictionary && message.dictionary.length))
                                    message.dictionary = [];
                                message.dictionary.push($root.doramcap.telemetry.v1.DictionaryEntry.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 11: {
                                if (wireType !== 2)
                                    break;
                                message.viewerIngress = $root.doramcap.telemetry.v1.Ingress.decode(reader, reader.size(), $undefined, _depth + 1, message.viewerIngress);
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
                 * Creates a SnapshotFrame message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.telemetry.v1.SnapshotFrame} SnapshotFrame
                 */
                SnapshotFrame.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.telemetry.v1.SnapshotFrame)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.telemetry.v1.SnapshotFrame: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.telemetry.v1.SnapshotFrame();
                    if (object.version != null)
                        if ($Number(object.version) !== 0)
                            message.version = object.version >>> 0;
                    if (object.snapshot != null) {
                        if (!$util.isObject(object.snapshot))
                            throw $TypeError(".doramcap.telemetry.v1.SnapshotFrame.snapshot: object expected");
                        message.snapshot = $root.doramcap.telemetry.v1.Snapshot.fromObject(object.snapshot, _depth + 1);
                    }
                    if (object.source != null)
                        if (typeof object.source !== "string" || object.source.length)
                            message.source = $String(object.source);
                    if (object.taskCollectionEnabled != null)
                        if (object.taskCollectionEnabled)
                            message.taskCollectionEnabled = $Boolean(object.taskCollectionEnabled);
                    if (object.taskCapacity != null)
                        if (typeof object.taskCapacity === "object" ? object.taskCapacity.low || object.taskCapacity.high : $Number(object.taskCapacity) !== 0)
                            if ($util.Long)
                                message.taskCapacity = $util.Long.fromValue(object.taskCapacity, true);
                            else if (typeof object.taskCapacity === "string")
                                message.taskCapacity = $parseInt(object.taskCapacity, 10);
                            else if (typeof object.taskCapacity === "number")
                                message.taskCapacity = object.taskCapacity;
                            else if (typeof object.taskCapacity === "object")
                                message.taskCapacity = new $util.LongBits(object.taskCapacity.low >>> 0, object.taskCapacity.high >>> 0).toNumber(true);
                    if (object.sessionId != null)
                        if (typeof object.sessionId !== "string" || object.sessionId.length)
                            message.sessionId = $String(object.sessionId);
                    if (object.dictionaryRevision != null)
                        if (typeof object.dictionaryRevision === "object" ? object.dictionaryRevision.low || object.dictionaryRevision.high : $Number(object.dictionaryRevision) !== 0)
                            if ($util.Long)
                                message.dictionaryRevision = $util.Long.fromValue(object.dictionaryRevision, true);
                            else if (typeof object.dictionaryRevision === "string")
                                message.dictionaryRevision = $parseInt(object.dictionaryRevision, 10);
                            else if (typeof object.dictionaryRevision === "number")
                                message.dictionaryRevision = object.dictionaryRevision;
                            else if (typeof object.dictionaryRevision === "object")
                                message.dictionaryRevision = new $util.LongBits(object.dictionaryRevision.low >>> 0, object.dictionaryRevision.high >>> 0).toNumber(true);
                    if (object.dictionaryFull != null)
                        if (object.dictionaryFull)
                            message.dictionaryFull = $Boolean(object.dictionaryFull);
                    if (object.dictionaryOverflow != null)
                        if (typeof object.dictionaryOverflow === "object" ? object.dictionaryOverflow.low || object.dictionaryOverflow.high : $Number(object.dictionaryOverflow) !== 0)
                            if ($util.Long)
                                message.dictionaryOverflow = $util.Long.fromValue(object.dictionaryOverflow, true);
                            else if (typeof object.dictionaryOverflow === "string")
                                message.dictionaryOverflow = $parseInt(object.dictionaryOverflow, 10);
                            else if (typeof object.dictionaryOverflow === "number")
                                message.dictionaryOverflow = object.dictionaryOverflow;
                            else if (typeof object.dictionaryOverflow === "object")
                                message.dictionaryOverflow = new $util.LongBits(object.dictionaryOverflow.low >>> 0, object.dictionaryOverflow.high >>> 0).toNumber(true);
                    if (object.dictionary) {
                        if (!$Array.isArray(object.dictionary))
                            throw $TypeError(".doramcap.telemetry.v1.SnapshotFrame.dictionary: array expected");
                        message.dictionary = $Array(object.dictionary.length);
                        for (let i = 0; i < object.dictionary.length; ++i) {
                            if (!$util.isObject(object.dictionary[i]))
                                throw $TypeError(".doramcap.telemetry.v1.SnapshotFrame.dictionary: object expected");
                            message.dictionary[i] = $root.doramcap.telemetry.v1.DictionaryEntry.fromObject(object.dictionary[i], _depth + 1);
                        }
                    }
                    if (object.viewerIngress != null) {
                        if (!$util.isObject(object.viewerIngress))
                            throw $TypeError(".doramcap.telemetry.v1.SnapshotFrame.viewerIngress: object expected");
                        message.viewerIngress = $root.doramcap.telemetry.v1.Ingress.fromObject(object.viewerIngress, _depth + 1);
                    }
                    return message;
                };

                /**
                 * Creates a plain object from a SnapshotFrame message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @static
                 * @param {doramcap.telemetry.v1.SnapshotFrame} message SnapshotFrame
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                SnapshotFrame.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.arrays || options.defaults)
                        object.dictionary = [];
                    if (options.defaults) {
                        object.version = 0;
                        object.snapshot = null;
                        object.source = "";
                        object.taskCollectionEnabled = false;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.taskCapacity = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.taskCapacity = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.sessionId = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.dictionaryRevision = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.dictionaryRevision = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.dictionaryFull = false;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.dictionaryOverflow = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.dictionaryOverflow = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.viewerIngress = null;
                    }
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version"))
                        object.version = message.version;
                    if (message.snapshot != null && $Object.hasOwnProperty.call(message, "snapshot"))
                        object.snapshot = $root.doramcap.telemetry.v1.Snapshot.toObject(message.snapshot, options, _depth + 1);
                    if (message.source != null && $Object.hasOwnProperty.call(message, "source"))
                        object.source = message.source;
                    if (message.taskCollectionEnabled != null && $Object.hasOwnProperty.call(message, "taskCollectionEnabled"))
                        object.taskCollectionEnabled = message.taskCollectionEnabled;
                    if (message.taskCapacity != null && $Object.hasOwnProperty.call(message, "taskCapacity"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.taskCapacity = typeof message.taskCapacity === "number" ? $BigInt(message.taskCapacity) : $util.Long.fromBits(message.taskCapacity.low >>> 0, message.taskCapacity.high >>> 0, true).toBigInt();
                        else if (typeof message.taskCapacity === "number")
                            object.taskCapacity = options.longs === $String ? $String(message.taskCapacity) : message.taskCapacity;
                        else
                            object.taskCapacity = options.longs === $String ? $util.Long.prototype.toString.call(message.taskCapacity) : options.longs === $Number ? new $util.LongBits(message.taskCapacity.low >>> 0, message.taskCapacity.high >>> 0).toNumber(true) : message.taskCapacity;
                    if (message.sessionId != null && $Object.hasOwnProperty.call(message, "sessionId"))
                        object.sessionId = message.sessionId;
                    if (message.dictionaryRevision != null && $Object.hasOwnProperty.call(message, "dictionaryRevision"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.dictionaryRevision = typeof message.dictionaryRevision === "number" ? $BigInt(message.dictionaryRevision) : $util.Long.fromBits(message.dictionaryRevision.low >>> 0, message.dictionaryRevision.high >>> 0, true).toBigInt();
                        else if (typeof message.dictionaryRevision === "number")
                            object.dictionaryRevision = options.longs === $String ? $String(message.dictionaryRevision) : message.dictionaryRevision;
                        else
                            object.dictionaryRevision = options.longs === $String ? $util.Long.prototype.toString.call(message.dictionaryRevision) : options.longs === $Number ? new $util.LongBits(message.dictionaryRevision.low >>> 0, message.dictionaryRevision.high >>> 0).toNumber(true) : message.dictionaryRevision;
                    if (message.dictionaryFull != null && $Object.hasOwnProperty.call(message, "dictionaryFull"))
                        object.dictionaryFull = message.dictionaryFull;
                    if (message.dictionaryOverflow != null && $Object.hasOwnProperty.call(message, "dictionaryOverflow"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.dictionaryOverflow = typeof message.dictionaryOverflow === "number" ? $BigInt(message.dictionaryOverflow) : $util.Long.fromBits(message.dictionaryOverflow.low >>> 0, message.dictionaryOverflow.high >>> 0, true).toBigInt();
                        else if (typeof message.dictionaryOverflow === "number")
                            object.dictionaryOverflow = options.longs === $String ? $String(message.dictionaryOverflow) : message.dictionaryOverflow;
                        else
                            object.dictionaryOverflow = options.longs === $String ? $util.Long.prototype.toString.call(message.dictionaryOverflow) : options.longs === $Number ? new $util.LongBits(message.dictionaryOverflow.low >>> 0, message.dictionaryOverflow.high >>> 0).toNumber(true) : message.dictionaryOverflow;
                    if (message.dictionary && message.dictionary.length) {
                        object.dictionary = $Array(message.dictionary.length);
                        for (let j = 0; j < message.dictionary.length; ++j)
                            object.dictionary[j] = $root.doramcap.telemetry.v1.DictionaryEntry.toObject(message.dictionary[j], options, _depth + 1);
                    }
                    if (message.viewerIngress != null && $Object.hasOwnProperty.call(message, "viewerIngress"))
                        object.viewerIngress = $root.doramcap.telemetry.v1.Ingress.toObject(message.viewerIngress, options, _depth + 1);
                    return object;
                };

                /**
                 * Converts this SnapshotFrame to JSON.
                 * @function toJSON
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                SnapshotFrame.prototype.toJSON = function() {
                    return SnapshotFrame.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for SnapshotFrame
                 * @function getTypeUrl
                 * @memberof doramcap.telemetry.v1.SnapshotFrame
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                SnapshotFrame.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.telemetry.v1.SnapshotFrame";
                };

                return SnapshotFrame;
            })();

            v1.DictionaryFrame = (function() {

                /**
                 * Properties of a DictionaryFrame.
                 * @typedef {Object} doramcap.telemetry.v1.DictionaryFrame.$Properties
                 * @property {number|null} [version] DictionaryFrame version
                 * @property {string|null} [sessionId] DictionaryFrame sessionId
                 * @property {number|Long|null} [revision] DictionaryFrame revision
                 * @property {number|Long|null} [overflow] DictionaryFrame overflow
                 * @property {Array.<doramcap.telemetry.v1.DictionaryEntry.$Properties>|null} [entries] DictionaryFrame entries
                 * @property {number|Long|null} [captureUnixNs] DictionaryFrame captureUnixNs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a DictionaryFrame.
                 * @memberof doramcap.telemetry.v1
                 * @interface IDictionaryFrame
                 * @augments doramcap.telemetry.v1.DictionaryFrame.$Properties
                 * @deprecated Use doramcap.telemetry.v1.DictionaryFrame.$Properties instead.
                 */

                /**
                 * Shape of a DictionaryFrame.
                 * @typedef {doramcap.telemetry.v1.DictionaryFrame.$Properties} doramcap.telemetry.v1.DictionaryFrame.$Shape
                 */

                /**
                 * Constructs a new DictionaryFrame.
                 * @memberof doramcap.telemetry.v1
                 * @classdesc Represents a DictionaryFrame.
                 * @constructor
                 * @param {doramcap.telemetry.v1.DictionaryFrame.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const DictionaryFrame = function (properties) {
                    this.entries = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * DictionaryFrame version.
                 * @member {number} version
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @instance
                 */
                DictionaryFrame.prototype.version = 0;

                /**
                 * DictionaryFrame sessionId.
                 * @member {string} sessionId
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @instance
                 */
                DictionaryFrame.prototype.sessionId = "";

                /**
                 * DictionaryFrame revision.
                 * @member {number|Long} revision
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @instance
                 */
                DictionaryFrame.prototype.revision = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * DictionaryFrame overflow.
                 * @member {number|Long} overflow
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @instance
                 */
                DictionaryFrame.prototype.overflow = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * DictionaryFrame entries.
                 * @member {Array.<doramcap.telemetry.v1.DictionaryEntry.$Properties>} entries
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @instance
                 */
                DictionaryFrame.prototype.entries = $util.emptyArray;

                /**
                 * DictionaryFrame captureUnixNs.
                 * @member {number|Long} captureUnixNs
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @instance
                 */
                DictionaryFrame.prototype.captureUnixNs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified DictionaryFrame message. Does not implicitly {@link doramcap.telemetry.v1.DictionaryFrame.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @static
                 * @param {doramcap.telemetry.v1.DictionaryFrame.$Properties} message DictionaryFrame message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                DictionaryFrame.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version") && message.version !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.version);
                    if (message.sessionId != null && $Object.hasOwnProperty.call(message, "sessionId") && message.sessionId !== "")
                        writer.uint32(/* id 2, wireType 2 =*/18).string(message.sessionId);
                    if (message.revision != null && $Object.hasOwnProperty.call(message, "revision") && (typeof message.revision === "object" ? message.revision.low || message.revision.high : message.revision !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.revision);
                    if (message.overflow != null && $Object.hasOwnProperty.call(message, "overflow") && (typeof message.overflow === "object" ? message.overflow.low || message.overflow.high : message.overflow !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.overflow);
                    if (message.entries != null && message.entries.length)
                        for (let i = 0; i < message.entries.length; ++i)
                            $root.doramcap.telemetry.v1.DictionaryEntry.encode(message.entries[i], writer.uint32(/* id 5, wireType 2 =*/42).fork(), _depth + 1).ldelim();
                    if (message.captureUnixNs != null && $Object.hasOwnProperty.call(message, "captureUnixNs") && (typeof message.captureUnixNs === "object" ? message.captureUnixNs.low || message.captureUnixNs.high : message.captureUnixNs !== 0))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.captureUnixNs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a DictionaryFrame message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.DictionaryFrame & doramcap.telemetry.v1.DictionaryFrame.$Shape} DictionaryFrame
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                DictionaryFrame.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.telemetry.v1.DictionaryFrame();
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
                                    message.sessionId = value;
                                else
                                    delete message.sessionId;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.revision = value;
                                else
                                    delete message.revision;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.overflow = value;
                                else
                                    delete message.overflow;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.entries && message.entries.length))
                                    message.entries = [];
                                message.entries.push($root.doramcap.telemetry.v1.DictionaryEntry.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.captureUnixNs = value;
                                else
                                    delete message.captureUnixNs;
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
                 * Creates a DictionaryFrame message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.telemetry.v1.DictionaryFrame} DictionaryFrame
                 */
                DictionaryFrame.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.telemetry.v1.DictionaryFrame)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.telemetry.v1.DictionaryFrame: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.telemetry.v1.DictionaryFrame();
                    if (object.version != null)
                        if ($Number(object.version) !== 0)
                            message.version = object.version >>> 0;
                    if (object.sessionId != null)
                        if (typeof object.sessionId !== "string" || object.sessionId.length)
                            message.sessionId = $String(object.sessionId);
                    if (object.revision != null)
                        if (typeof object.revision === "object" ? object.revision.low || object.revision.high : $Number(object.revision) !== 0)
                            if ($util.Long)
                                message.revision = $util.Long.fromValue(object.revision, true);
                            else if (typeof object.revision === "string")
                                message.revision = $parseInt(object.revision, 10);
                            else if (typeof object.revision === "number")
                                message.revision = object.revision;
                            else if (typeof object.revision === "object")
                                message.revision = new $util.LongBits(object.revision.low >>> 0, object.revision.high >>> 0).toNumber(true);
                    if (object.overflow != null)
                        if (typeof object.overflow === "object" ? object.overflow.low || object.overflow.high : $Number(object.overflow) !== 0)
                            if ($util.Long)
                                message.overflow = $util.Long.fromValue(object.overflow, true);
                            else if (typeof object.overflow === "string")
                                message.overflow = $parseInt(object.overflow, 10);
                            else if (typeof object.overflow === "number")
                                message.overflow = object.overflow;
                            else if (typeof object.overflow === "object")
                                message.overflow = new $util.LongBits(object.overflow.low >>> 0, object.overflow.high >>> 0).toNumber(true);
                    if (object.entries) {
                        if (!$Array.isArray(object.entries))
                            throw $TypeError(".doramcap.telemetry.v1.DictionaryFrame.entries: array expected");
                        message.entries = $Array(object.entries.length);
                        for (let i = 0; i < object.entries.length; ++i) {
                            if (!$util.isObject(object.entries[i]))
                                throw $TypeError(".doramcap.telemetry.v1.DictionaryFrame.entries: object expected");
                            message.entries[i] = $root.doramcap.telemetry.v1.DictionaryEntry.fromObject(object.entries[i], _depth + 1);
                        }
                    }
                    if (object.captureUnixNs != null)
                        if (typeof object.captureUnixNs === "object" ? object.captureUnixNs.low || object.captureUnixNs.high : $Number(object.captureUnixNs) !== 0)
                            if ($util.Long)
                                message.captureUnixNs = $util.Long.fromValue(object.captureUnixNs, true);
                            else if (typeof object.captureUnixNs === "string")
                                message.captureUnixNs = $parseInt(object.captureUnixNs, 10);
                            else if (typeof object.captureUnixNs === "number")
                                message.captureUnixNs = object.captureUnixNs;
                            else if (typeof object.captureUnixNs === "object")
                                message.captureUnixNs = new $util.LongBits(object.captureUnixNs.low >>> 0, object.captureUnixNs.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a DictionaryFrame message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @static
                 * @param {doramcap.telemetry.v1.DictionaryFrame} message DictionaryFrame
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                DictionaryFrame.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.arrays || options.defaults)
                        object.entries = [];
                    if (options.defaults) {
                        object.version = 0;
                        object.sessionId = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.revision = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.revision = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.overflow = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.overflow = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.captureUnixNs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.captureUnixNs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version"))
                        object.version = message.version;
                    if (message.sessionId != null && $Object.hasOwnProperty.call(message, "sessionId"))
                        object.sessionId = message.sessionId;
                    if (message.revision != null && $Object.hasOwnProperty.call(message, "revision"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.revision = typeof message.revision === "number" ? $BigInt(message.revision) : $util.Long.fromBits(message.revision.low >>> 0, message.revision.high >>> 0, true).toBigInt();
                        else if (typeof message.revision === "number")
                            object.revision = options.longs === $String ? $String(message.revision) : message.revision;
                        else
                            object.revision = options.longs === $String ? $util.Long.prototype.toString.call(message.revision) : options.longs === $Number ? new $util.LongBits(message.revision.low >>> 0, message.revision.high >>> 0).toNumber(true) : message.revision;
                    if (message.overflow != null && $Object.hasOwnProperty.call(message, "overflow"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.overflow = typeof message.overflow === "number" ? $BigInt(message.overflow) : $util.Long.fromBits(message.overflow.low >>> 0, message.overflow.high >>> 0, true).toBigInt();
                        else if (typeof message.overflow === "number")
                            object.overflow = options.longs === $String ? $String(message.overflow) : message.overflow;
                        else
                            object.overflow = options.longs === $String ? $util.Long.prototype.toString.call(message.overflow) : options.longs === $Number ? new $util.LongBits(message.overflow.low >>> 0, message.overflow.high >>> 0).toNumber(true) : message.overflow;
                    if (message.entries && message.entries.length) {
                        object.entries = $Array(message.entries.length);
                        for (let j = 0; j < message.entries.length; ++j)
                            object.entries[j] = $root.doramcap.telemetry.v1.DictionaryEntry.toObject(message.entries[j], options, _depth + 1);
                    }
                    if (message.captureUnixNs != null && $Object.hasOwnProperty.call(message, "captureUnixNs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.captureUnixNs = typeof message.captureUnixNs === "number" ? $BigInt(message.captureUnixNs) : $util.Long.fromBits(message.captureUnixNs.low >>> 0, message.captureUnixNs.high >>> 0, true).toBigInt();
                        else if (typeof message.captureUnixNs === "number")
                            object.captureUnixNs = options.longs === $String ? $String(message.captureUnixNs) : message.captureUnixNs;
                        else
                            object.captureUnixNs = options.longs === $String ? $util.Long.prototype.toString.call(message.captureUnixNs) : options.longs === $Number ? new $util.LongBits(message.captureUnixNs.low >>> 0, message.captureUnixNs.high >>> 0).toNumber(true) : message.captureUnixNs;
                    return object;
                };

                /**
                 * Converts this DictionaryFrame to JSON.
                 * @function toJSON
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                DictionaryFrame.prototype.toJSON = function() {
                    return DictionaryFrame.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for DictionaryFrame
                 * @function getTypeUrl
                 * @memberof doramcap.telemetry.v1.DictionaryFrame
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                DictionaryFrame.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.telemetry.v1.DictionaryFrame";
                };

                return DictionaryFrame;
            })();

            return v1;
        })();

        return telemetry;
    })();

    return doramcap;
})();

export {
  $root as default
};
