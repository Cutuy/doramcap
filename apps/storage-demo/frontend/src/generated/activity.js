/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-mixed-operators, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars, default-case, jsdoc/require-param*/
import $protobuf from "protobufjs/minimal.js";

// Common aliases
const $Reader = $protobuf.Reader, $Writer = $protobuf.Writer, $util = $protobuf.util;
const $Object = $util.global.Object, $undefined = $util.global.undefined, $Error = $util.global.Error, $RangeError = $util.global.RangeError, $TypeError = $util.global.TypeError, $Number = $util.global.Number, $parseInt = $util.global.parseInt, $Array = $util.global.Array, $String = $util.global.String, $BigInt = $util.global.BigInt;

// Exported root namespace
const $root = $protobuf.roots["default"] || ($protobuf.roots["default"] = {});

export const doramcap = $root.doramcap = (() => {

    /**
     * Namespace doramcap.
     * @exports doramcap
     * @namespace
     */
    const doramcap = {};

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
