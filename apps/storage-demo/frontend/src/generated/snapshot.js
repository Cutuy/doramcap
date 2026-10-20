/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-mixed-operators, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars, default-case, jsdoc/require-param*/
import $protobuf from "protobufjs/minimal.js";

// Common aliases
const $Reader = $protobuf.Reader, $Writer = $protobuf.Writer, $util = $protobuf.util;
const $Object = $util.global.Object, $undefined = $util.global.undefined, $Error = $util.global.Error, $RangeError = $util.global.RangeError, $TypeError = $util.global.TypeError, $Number = $util.global.Number, $String = $util.global.String, $Boolean = $util.global.Boolean, $parseInt = $util.global.parseInt, $Array = $util.global.Array, $BigInt = $util.global.BigInt, $isFinite = $util.global.isFinite;

// Exported root namespace
const $root = $protobuf.roots["storageSnapshots"] || ($protobuf.roots["storageSnapshots"] = {});

export const doramcap = $root.doramcap = (() => {

    /**
     * Namespace doramcap.
     * @exports doramcap
     * @namespace
     */
    const doramcap = {};

    doramcap.storage = (function() {

        /**
         * Namespace storage.
         * @memberof doramcap
         * @namespace
         */
        const storage = {};

        storage.v1 = (function() {

            /**
             * Namespace v1.
             * @memberof doramcap.storage
             * @namespace
             */
            const v1 = {};

            v1.Snapshot = (function() {

                /**
                 * Properties of a Snapshot.
                 * @typedef {Object} doramcap.storage.v1.Snapshot.$Properties
                 * @property {number|null} [version] Snapshot version
                 * @property {string|null} [status] Snapshot status
                 * @property {string|null} [session] Snapshot session
                 * @property {string|null} [error] Snapshot error
                 * @property {string|null} [backend] Snapshot backend
                 * @property {boolean|null} [direct] Snapshot direct
                 * @property {number|Long|null} [elapsedMs] Snapshot elapsedMs
                 * @property {number|Long|null} [cpu] Snapshot cpu
                 * @property {number|Long|null} [heartbeats] Snapshot heartbeats
                 * @property {number|Long|null} [probes] Snapshot probes
                 * @property {number|Long|null} [inputDrops] Snapshot inputDrops
                 * @property {doramcap.storage.v1.LastCpu.$Properties|null} [lastCpu] Snapshot lastCpu
                 * @property {doramcap.storage.v1.Combining.$Properties|null} [combining] Snapshot combining
                 * @property {doramcap.storage.v1.Pool.$Properties|null} [pool] Snapshot pool
                 * @property {doramcap.storage.v1.Ring.$Properties|null} [ring] Snapshot ring
                 * @property {doramcap.storage.v1.Exercises.$Properties|null} [exercises] Snapshot exercises
                 * @property {doramcap.storage.v1.Observation.$Properties|null} [observation] Snapshot observation
                 * @property {Array.<doramcap.storage.v1.ActiveWrite.$Properties>|null} [activeWrites] Snapshot activeWrites
                 * @property {doramcap.storage.v1.Timing.$Properties|null} [timing] Snapshot timing
                 * @property {doramcap.storage.v1.Stats.$Properties|null} [latency] Snapshot latency
                 * @property {number|Long|null} [lostTimingEvents] Snapshot lostTimingEvents
                 * @property {number|Long|null} [burstRemaining] Snapshot burstRemaining
                 * @property {Array.<doramcap.storage.v1.Receipt.$Properties>|null} [receipts] Snapshot receipts
                 * @property {Array.<doramcap.storage.v1.Input.$Properties>|null} [messages] Snapshot messages
                 * @property {Array.<doramcap.storage.v1.Event.$Properties>|null} [events] Snapshot events
                 * @property {doramcap.storage.v1.File.$Properties|null} [file] Snapshot file
                 * @property {doramcap.storage.v1.File.$Properties|null} [lastSealed] Snapshot lastSealed
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Snapshot.
                 * @memberof doramcap.storage.v1
                 * @interface ISnapshot
                 * @augments doramcap.storage.v1.Snapshot.$Properties
                 * @deprecated Use doramcap.storage.v1.Snapshot.$Properties instead.
                 */

                /**
                 * Shape of a Snapshot.
                 * @typedef {doramcap.storage.v1.Snapshot.$Properties} doramcap.storage.v1.Snapshot.$Shape
                 */

                /**
                 * Constructs a new Snapshot.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a Snapshot.
                 * @constructor
                 * @param {doramcap.storage.v1.Snapshot.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Snapshot = function (properties) {
                    this.activeWrites = [];
                    this.receipts = [];
                    this.messages = [];
                    this.events = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Snapshot version.
                 * @member {number} version
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.version = 0;

                /**
                 * Snapshot status.
                 * @member {string} status
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.status = "";

                /**
                 * Snapshot session.
                 * @member {string} session
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.session = "";

                /**
                 * Snapshot error.
                 * @member {string} error
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.error = "";

                /**
                 * Snapshot backend.
                 * @member {string} backend
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.backend = "";

                /**
                 * Snapshot direct.
                 * @member {boolean} direct
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.direct = false;

                /**
                 * Snapshot elapsedMs.
                 * @member {number|Long} elapsedMs
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.elapsedMs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot cpu.
                 * @member {number|Long} cpu
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.cpu = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot heartbeats.
                 * @member {number|Long} heartbeats
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.heartbeats = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot probes.
                 * @member {number|Long} probes
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.probes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot inputDrops.
                 * @member {number|Long} inputDrops
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.inputDrops = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot lastCpu.
                 * @member {doramcap.storage.v1.LastCpu.$Properties|null|undefined} lastCpu
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.lastCpu = null;

                /**
                 * Snapshot combining.
                 * @member {doramcap.storage.v1.Combining.$Properties|null|undefined} combining
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.combining = null;

                /**
                 * Snapshot pool.
                 * @member {doramcap.storage.v1.Pool.$Properties|null|undefined} pool
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.pool = null;

                /**
                 * Snapshot ring.
                 * @member {doramcap.storage.v1.Ring.$Properties|null|undefined} ring
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.ring = null;

                /**
                 * Snapshot exercises.
                 * @member {doramcap.storage.v1.Exercises.$Properties|null|undefined} exercises
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.exercises = null;

                /**
                 * Snapshot observation.
                 * @member {doramcap.storage.v1.Observation.$Properties|null|undefined} observation
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.observation = null;

                /**
                 * Snapshot activeWrites.
                 * @member {Array.<doramcap.storage.v1.ActiveWrite.$Properties>} activeWrites
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.activeWrites = $util.emptyArray;

                /**
                 * Snapshot timing.
                 * @member {doramcap.storage.v1.Timing.$Properties|null|undefined} timing
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.timing = null;

                /**
                 * Snapshot latency.
                 * @member {doramcap.storage.v1.Stats.$Properties|null|undefined} latency
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.latency = null;

                /**
                 * Snapshot lostTimingEvents.
                 * @member {number|Long} lostTimingEvents
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.lostTimingEvents = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot burstRemaining.
                 * @member {number|Long} burstRemaining
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.burstRemaining = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Snapshot receipts.
                 * @member {Array.<doramcap.storage.v1.Receipt.$Properties>} receipts
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.receipts = $util.emptyArray;

                /**
                 * Snapshot messages.
                 * @member {Array.<doramcap.storage.v1.Input.$Properties>} messages
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.messages = $util.emptyArray;

                /**
                 * Snapshot events.
                 * @member {Array.<doramcap.storage.v1.Event.$Properties>} events
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.events = $util.emptyArray;

                /**
                 * Snapshot file.
                 * @member {doramcap.storage.v1.File.$Properties|null|undefined} file
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.file = null;

                /**
                 * Snapshot lastSealed.
                 * @member {doramcap.storage.v1.File.$Properties|null|undefined} lastSealed
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 */
                Snapshot.prototype.lastSealed = null;

                /**
                 * Encodes the specified Snapshot message. Does not implicitly {@link doramcap.storage.v1.Snapshot.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Snapshot
                 * @static
                 * @param {doramcap.storage.v1.Snapshot.$Properties} message Snapshot message or plain object to encode
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
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version") && message.version !== 0)
                        writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.version);
                    if (message.status != null && $Object.hasOwnProperty.call(message, "status") && message.status !== "")
                        writer.uint32(/* id 2, wireType 2 =*/18).string(message.status);
                    if (message.session != null && $Object.hasOwnProperty.call(message, "session") && message.session !== "")
                        writer.uint32(/* id 3, wireType 2 =*/26).string(message.session);
                    if (message.error != null && $Object.hasOwnProperty.call(message, "error") && message.error !== "")
                        writer.uint32(/* id 4, wireType 2 =*/34).string(message.error);
                    if (message.backend != null && $Object.hasOwnProperty.call(message, "backend") && message.backend !== "")
                        writer.uint32(/* id 5, wireType 2 =*/42).string(message.backend);
                    if (message.direct != null && $Object.hasOwnProperty.call(message, "direct") && message.direct !== false)
                        writer.uint32(/* id 6, wireType 0 =*/48).bool(message.direct);
                    if (message.elapsedMs != null && $Object.hasOwnProperty.call(message, "elapsedMs") && (typeof message.elapsedMs === "object" ? message.elapsedMs.low || message.elapsedMs.high : message.elapsedMs !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.elapsedMs);
                    if (message.cpu != null && $Object.hasOwnProperty.call(message, "cpu") && (typeof message.cpu === "object" ? message.cpu.low || message.cpu.high : message.cpu !== 0))
                        writer.uint32(/* id 8, wireType 0 =*/64).uint64(message.cpu);
                    if (message.heartbeats != null && $Object.hasOwnProperty.call(message, "heartbeats") && (typeof message.heartbeats === "object" ? message.heartbeats.low || message.heartbeats.high : message.heartbeats !== 0))
                        writer.uint32(/* id 9, wireType 0 =*/72).uint64(message.heartbeats);
                    if (message.probes != null && $Object.hasOwnProperty.call(message, "probes") && (typeof message.probes === "object" ? message.probes.low || message.probes.high : message.probes !== 0))
                        writer.uint32(/* id 10, wireType 0 =*/80).uint64(message.probes);
                    if (message.inputDrops != null && $Object.hasOwnProperty.call(message, "inputDrops") && (typeof message.inputDrops === "object" ? message.inputDrops.low || message.inputDrops.high : message.inputDrops !== 0))
                        writer.uint32(/* id 11, wireType 0 =*/88).uint64(message.inputDrops);
                    if (message.lastCpu != null && $Object.hasOwnProperty.call(message, "lastCpu"))
                        $root.doramcap.storage.v1.LastCpu.encode(message.lastCpu, writer.uint32(/* id 12, wireType 2 =*/98).fork(), _depth + 1).ldelim();
                    if (message.combining != null && $Object.hasOwnProperty.call(message, "combining"))
                        $root.doramcap.storage.v1.Combining.encode(message.combining, writer.uint32(/* id 13, wireType 2 =*/106).fork(), _depth + 1).ldelim();
                    if (message.pool != null && $Object.hasOwnProperty.call(message, "pool"))
                        $root.doramcap.storage.v1.Pool.encode(message.pool, writer.uint32(/* id 14, wireType 2 =*/114).fork(), _depth + 1).ldelim();
                    if (message.ring != null && $Object.hasOwnProperty.call(message, "ring"))
                        $root.doramcap.storage.v1.Ring.encode(message.ring, writer.uint32(/* id 15, wireType 2 =*/122).fork(), _depth + 1).ldelim();
                    if (message.exercises != null && $Object.hasOwnProperty.call(message, "exercises"))
                        $root.doramcap.storage.v1.Exercises.encode(message.exercises, writer.uint32(/* id 16, wireType 2 =*/130).fork(), _depth + 1).ldelim();
                    if (message.observation != null && $Object.hasOwnProperty.call(message, "observation"))
                        $root.doramcap.storage.v1.Observation.encode(message.observation, writer.uint32(/* id 17, wireType 2 =*/138).fork(), _depth + 1).ldelim();
                    if (message.activeWrites != null && message.activeWrites.length)
                        for (let i = 0; i < message.activeWrites.length; ++i)
                            $root.doramcap.storage.v1.ActiveWrite.encode(message.activeWrites[i], writer.uint32(/* id 18, wireType 2 =*/146).fork(), _depth + 1).ldelim();
                    if (message.timing != null && $Object.hasOwnProperty.call(message, "timing"))
                        $root.doramcap.storage.v1.Timing.encode(message.timing, writer.uint32(/* id 19, wireType 2 =*/154).fork(), _depth + 1).ldelim();
                    if (message.latency != null && $Object.hasOwnProperty.call(message, "latency"))
                        $root.doramcap.storage.v1.Stats.encode(message.latency, writer.uint32(/* id 20, wireType 2 =*/162).fork(), _depth + 1).ldelim();
                    if (message.lostTimingEvents != null && $Object.hasOwnProperty.call(message, "lostTimingEvents") && (typeof message.lostTimingEvents === "object" ? message.lostTimingEvents.low || message.lostTimingEvents.high : message.lostTimingEvents !== 0))
                        writer.uint32(/* id 21, wireType 0 =*/168).uint64(message.lostTimingEvents);
                    if (message.burstRemaining != null && $Object.hasOwnProperty.call(message, "burstRemaining") && (typeof message.burstRemaining === "object" ? message.burstRemaining.low || message.burstRemaining.high : message.burstRemaining !== 0))
                        writer.uint32(/* id 22, wireType 0 =*/176).uint64(message.burstRemaining);
                    if (message.receipts != null && message.receipts.length)
                        for (let i = 0; i < message.receipts.length; ++i)
                            $root.doramcap.storage.v1.Receipt.encode(message.receipts[i], writer.uint32(/* id 23, wireType 2 =*/186).fork(), _depth + 1).ldelim();
                    if (message.messages != null && message.messages.length)
                        for (let i = 0; i < message.messages.length; ++i)
                            $root.doramcap.storage.v1.Input.encode(message.messages[i], writer.uint32(/* id 24, wireType 2 =*/194).fork(), _depth + 1).ldelim();
                    if (message.events != null && message.events.length)
                        for (let i = 0; i < message.events.length; ++i)
                            $root.doramcap.storage.v1.Event.encode(message.events[i], writer.uint32(/* id 25, wireType 2 =*/202).fork(), _depth + 1).ldelim();
                    if (message.file != null && $Object.hasOwnProperty.call(message, "file"))
                        $root.doramcap.storage.v1.File.encode(message.file, writer.uint32(/* id 26, wireType 2 =*/210).fork(), _depth + 1).ldelim();
                    if (message.lastSealed != null && $Object.hasOwnProperty.call(message, "lastSealed"))
                        $root.doramcap.storage.v1.File.encode(message.lastSealed, writer.uint32(/* id 27, wireType 2 =*/218).fork(), _depth + 1).ldelim();
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Snapshot message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Snapshot
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Snapshot & doramcap.storage.v1.Snapshot.$Shape} Snapshot
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
                    message = _target || new $root.doramcap.storage.v1.Snapshot();
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
                                    message.status = value;
                                else
                                    delete message.status;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.session = value;
                                else
                                    delete message.session;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.error = value;
                                else
                                    delete message.error;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.backend = value;
                                else
                                    delete message.backend;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.bool())
                                    message.direct = value;
                                else
                                    delete message.direct;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.elapsedMs = value;
                                else
                                    delete message.elapsedMs;
                                continue;
                            }
                        case 8: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.cpu = value;
                                else
                                    delete message.cpu;
                                continue;
                            }
                        case 9: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.heartbeats = value;
                                else
                                    delete message.heartbeats;
                                continue;
                            }
                        case 10: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.probes = value;
                                else
                                    delete message.probes;
                                continue;
                            }
                        case 11: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.inputDrops = value;
                                else
                                    delete message.inputDrops;
                                continue;
                            }
                        case 12: {
                                if (wireType !== 2)
                                    break;
                                message.lastCpu = $root.doramcap.storage.v1.LastCpu.decode(reader, reader.size(), $undefined, _depth + 1, message.lastCpu);
                                continue;
                            }
                        case 13: {
                                if (wireType !== 2)
                                    break;
                                message.combining = $root.doramcap.storage.v1.Combining.decode(reader, reader.size(), $undefined, _depth + 1, message.combining);
                                continue;
                            }
                        case 14: {
                                if (wireType !== 2)
                                    break;
                                message.pool = $root.doramcap.storage.v1.Pool.decode(reader, reader.size(), $undefined, _depth + 1, message.pool);
                                continue;
                            }
                        case 15: {
                                if (wireType !== 2)
                                    break;
                                message.ring = $root.doramcap.storage.v1.Ring.decode(reader, reader.size(), $undefined, _depth + 1, message.ring);
                                continue;
                            }
                        case 16: {
                                if (wireType !== 2)
                                    break;
                                message.exercises = $root.doramcap.storage.v1.Exercises.decode(reader, reader.size(), $undefined, _depth + 1, message.exercises);
                                continue;
                            }
                        case 17: {
                                if (wireType !== 2)
                                    break;
                                message.observation = $root.doramcap.storage.v1.Observation.decode(reader, reader.size(), $undefined, _depth + 1, message.observation);
                                continue;
                            }
                        case 18: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.activeWrites && message.activeWrites.length))
                                    message.activeWrites = [];
                                message.activeWrites.push($root.doramcap.storage.v1.ActiveWrite.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 19: {
                                if (wireType !== 2)
                                    break;
                                message.timing = $root.doramcap.storage.v1.Timing.decode(reader, reader.size(), $undefined, _depth + 1, message.timing);
                                continue;
                            }
                        case 20: {
                                if (wireType !== 2)
                                    break;
                                message.latency = $root.doramcap.storage.v1.Stats.decode(reader, reader.size(), $undefined, _depth + 1, message.latency);
                                continue;
                            }
                        case 21: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.lostTimingEvents = value;
                                else
                                    delete message.lostTimingEvents;
                                continue;
                            }
                        case 22: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.burstRemaining = value;
                                else
                                    delete message.burstRemaining;
                                continue;
                            }
                        case 23: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.receipts && message.receipts.length))
                                    message.receipts = [];
                                message.receipts.push($root.doramcap.storage.v1.Receipt.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 24: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.messages && message.messages.length))
                                    message.messages = [];
                                message.messages.push($root.doramcap.storage.v1.Input.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 25: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.events && message.events.length))
                                    message.events = [];
                                message.events.push($root.doramcap.storage.v1.Event.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 26: {
                                if (wireType !== 2)
                                    break;
                                message.file = $root.doramcap.storage.v1.File.decode(reader, reader.size(), $undefined, _depth + 1, message.file);
                                continue;
                            }
                        case 27: {
                                if (wireType !== 2)
                                    break;
                                message.lastSealed = $root.doramcap.storage.v1.File.decode(reader, reader.size(), $undefined, _depth + 1, message.lastSealed);
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
                 * @memberof doramcap.storage.v1.Snapshot
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Snapshot} Snapshot
                 */
                Snapshot.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Snapshot)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Snapshot: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Snapshot();
                    if (object.version != null)
                        if ($Number(object.version) !== 0)
                            message.version = object.version >>> 0;
                    if (object.status != null)
                        if (typeof object.status !== "string" || object.status.length)
                            message.status = $String(object.status);
                    if (object.session != null)
                        if (typeof object.session !== "string" || object.session.length)
                            message.session = $String(object.session);
                    if (object.error != null)
                        if (typeof object.error !== "string" || object.error.length)
                            message.error = $String(object.error);
                    if (object.backend != null)
                        if (typeof object.backend !== "string" || object.backend.length)
                            message.backend = $String(object.backend);
                    if (object.direct != null)
                        if (object.direct)
                            message.direct = $Boolean(object.direct);
                    if (object.elapsedMs != null)
                        if (typeof object.elapsedMs === "object" ? object.elapsedMs.low || object.elapsedMs.high : $Number(object.elapsedMs) !== 0)
                            if ($util.Long)
                                message.elapsedMs = $util.Long.fromValue(object.elapsedMs, true);
                            else if (typeof object.elapsedMs === "string")
                                message.elapsedMs = $parseInt(object.elapsedMs, 10);
                            else if (typeof object.elapsedMs === "number")
                                message.elapsedMs = object.elapsedMs;
                            else if (typeof object.elapsedMs === "object")
                                message.elapsedMs = new $util.LongBits(object.elapsedMs.low >>> 0, object.elapsedMs.high >>> 0).toNumber(true);
                    if (object.cpu != null)
                        if (typeof object.cpu === "object" ? object.cpu.low || object.cpu.high : $Number(object.cpu) !== 0)
                            if ($util.Long)
                                message.cpu = $util.Long.fromValue(object.cpu, true);
                            else if (typeof object.cpu === "string")
                                message.cpu = $parseInt(object.cpu, 10);
                            else if (typeof object.cpu === "number")
                                message.cpu = object.cpu;
                            else if (typeof object.cpu === "object")
                                message.cpu = new $util.LongBits(object.cpu.low >>> 0, object.cpu.high >>> 0).toNumber(true);
                    if (object.heartbeats != null)
                        if (typeof object.heartbeats === "object" ? object.heartbeats.low || object.heartbeats.high : $Number(object.heartbeats) !== 0)
                            if ($util.Long)
                                message.heartbeats = $util.Long.fromValue(object.heartbeats, true);
                            else if (typeof object.heartbeats === "string")
                                message.heartbeats = $parseInt(object.heartbeats, 10);
                            else if (typeof object.heartbeats === "number")
                                message.heartbeats = object.heartbeats;
                            else if (typeof object.heartbeats === "object")
                                message.heartbeats = new $util.LongBits(object.heartbeats.low >>> 0, object.heartbeats.high >>> 0).toNumber(true);
                    if (object.probes != null)
                        if (typeof object.probes === "object" ? object.probes.low || object.probes.high : $Number(object.probes) !== 0)
                            if ($util.Long)
                                message.probes = $util.Long.fromValue(object.probes, true);
                            else if (typeof object.probes === "string")
                                message.probes = $parseInt(object.probes, 10);
                            else if (typeof object.probes === "number")
                                message.probes = object.probes;
                            else if (typeof object.probes === "object")
                                message.probes = new $util.LongBits(object.probes.low >>> 0, object.probes.high >>> 0).toNumber(true);
                    if (object.inputDrops != null)
                        if (typeof object.inputDrops === "object" ? object.inputDrops.low || object.inputDrops.high : $Number(object.inputDrops) !== 0)
                            if ($util.Long)
                                message.inputDrops = $util.Long.fromValue(object.inputDrops, true);
                            else if (typeof object.inputDrops === "string")
                                message.inputDrops = $parseInt(object.inputDrops, 10);
                            else if (typeof object.inputDrops === "number")
                                message.inputDrops = object.inputDrops;
                            else if (typeof object.inputDrops === "object")
                                message.inputDrops = new $util.LongBits(object.inputDrops.low >>> 0, object.inputDrops.high >>> 0).toNumber(true);
                    if (object.lastCpu != null) {
                        if (!$util.isObject(object.lastCpu))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.lastCpu: object expected");
                        message.lastCpu = $root.doramcap.storage.v1.LastCpu.fromObject(object.lastCpu, _depth + 1);
                    }
                    if (object.combining != null) {
                        if (!$util.isObject(object.combining))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.combining: object expected");
                        message.combining = $root.doramcap.storage.v1.Combining.fromObject(object.combining, _depth + 1);
                    }
                    if (object.pool != null) {
                        if (!$util.isObject(object.pool))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.pool: object expected");
                        message.pool = $root.doramcap.storage.v1.Pool.fromObject(object.pool, _depth + 1);
                    }
                    if (object.ring != null) {
                        if (!$util.isObject(object.ring))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.ring: object expected");
                        message.ring = $root.doramcap.storage.v1.Ring.fromObject(object.ring, _depth + 1);
                    }
                    if (object.exercises != null) {
                        if (!$util.isObject(object.exercises))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.exercises: object expected");
                        message.exercises = $root.doramcap.storage.v1.Exercises.fromObject(object.exercises, _depth + 1);
                    }
                    if (object.observation != null) {
                        if (!$util.isObject(object.observation))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.observation: object expected");
                        message.observation = $root.doramcap.storage.v1.Observation.fromObject(object.observation, _depth + 1);
                    }
                    if (object.activeWrites) {
                        if (!$Array.isArray(object.activeWrites))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.activeWrites: array expected");
                        message.activeWrites = $Array(object.activeWrites.length);
                        for (let i = 0; i < object.activeWrites.length; ++i) {
                            if (!$util.isObject(object.activeWrites[i]))
                                throw $TypeError(".doramcap.storage.v1.Snapshot.activeWrites: object expected");
                            message.activeWrites[i] = $root.doramcap.storage.v1.ActiveWrite.fromObject(object.activeWrites[i], _depth + 1);
                        }
                    }
                    if (object.timing != null) {
                        if (!$util.isObject(object.timing))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.timing: object expected");
                        message.timing = $root.doramcap.storage.v1.Timing.fromObject(object.timing, _depth + 1);
                    }
                    if (object.latency != null) {
                        if (!$util.isObject(object.latency))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.latency: object expected");
                        message.latency = $root.doramcap.storage.v1.Stats.fromObject(object.latency, _depth + 1);
                    }
                    if (object.lostTimingEvents != null)
                        if (typeof object.lostTimingEvents === "object" ? object.lostTimingEvents.low || object.lostTimingEvents.high : $Number(object.lostTimingEvents) !== 0)
                            if ($util.Long)
                                message.lostTimingEvents = $util.Long.fromValue(object.lostTimingEvents, true);
                            else if (typeof object.lostTimingEvents === "string")
                                message.lostTimingEvents = $parseInt(object.lostTimingEvents, 10);
                            else if (typeof object.lostTimingEvents === "number")
                                message.lostTimingEvents = object.lostTimingEvents;
                            else if (typeof object.lostTimingEvents === "object")
                                message.lostTimingEvents = new $util.LongBits(object.lostTimingEvents.low >>> 0, object.lostTimingEvents.high >>> 0).toNumber(true);
                    if (object.burstRemaining != null)
                        if (typeof object.burstRemaining === "object" ? object.burstRemaining.low || object.burstRemaining.high : $Number(object.burstRemaining) !== 0)
                            if ($util.Long)
                                message.burstRemaining = $util.Long.fromValue(object.burstRemaining, true);
                            else if (typeof object.burstRemaining === "string")
                                message.burstRemaining = $parseInt(object.burstRemaining, 10);
                            else if (typeof object.burstRemaining === "number")
                                message.burstRemaining = object.burstRemaining;
                            else if (typeof object.burstRemaining === "object")
                                message.burstRemaining = new $util.LongBits(object.burstRemaining.low >>> 0, object.burstRemaining.high >>> 0).toNumber(true);
                    if (object.receipts) {
                        if (!$Array.isArray(object.receipts))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.receipts: array expected");
                        message.receipts = $Array(object.receipts.length);
                        for (let i = 0; i < object.receipts.length; ++i) {
                            if (!$util.isObject(object.receipts[i]))
                                throw $TypeError(".doramcap.storage.v1.Snapshot.receipts: object expected");
                            message.receipts[i] = $root.doramcap.storage.v1.Receipt.fromObject(object.receipts[i], _depth + 1);
                        }
                    }
                    if (object.messages) {
                        if (!$Array.isArray(object.messages))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.messages: array expected");
                        message.messages = $Array(object.messages.length);
                        for (let i = 0; i < object.messages.length; ++i) {
                            if (!$util.isObject(object.messages[i]))
                                throw $TypeError(".doramcap.storage.v1.Snapshot.messages: object expected");
                            message.messages[i] = $root.doramcap.storage.v1.Input.fromObject(object.messages[i], _depth + 1);
                        }
                    }
                    if (object.events) {
                        if (!$Array.isArray(object.events))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.events: array expected");
                        message.events = $Array(object.events.length);
                        for (let i = 0; i < object.events.length; ++i) {
                            if (!$util.isObject(object.events[i]))
                                throw $TypeError(".doramcap.storage.v1.Snapshot.events: object expected");
                            message.events[i] = $root.doramcap.storage.v1.Event.fromObject(object.events[i], _depth + 1);
                        }
                    }
                    if (object.file != null) {
                        if (!$util.isObject(object.file))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.file: object expected");
                        message.file = $root.doramcap.storage.v1.File.fromObject(object.file, _depth + 1);
                    }
                    if (object.lastSealed != null) {
                        if (!$util.isObject(object.lastSealed))
                            throw $TypeError(".doramcap.storage.v1.Snapshot.lastSealed: object expected");
                        message.lastSealed = $root.doramcap.storage.v1.File.fromObject(object.lastSealed, _depth + 1);
                    }
                    return message;
                };

                /**
                 * Creates a plain object from a Snapshot message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Snapshot
                 * @static
                 * @param {doramcap.storage.v1.Snapshot} message Snapshot
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
                        object.activeWrites = [];
                        object.receipts = [];
                        object.messages = [];
                        object.events = [];
                    }
                    if (options.defaults) {
                        object.version = 0;
                        object.status = "";
                        object.session = "";
                        object.error = "";
                        object.backend = "";
                        object.direct = false;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.elapsedMs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.elapsedMs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.cpu = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.cpu = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.heartbeats = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.heartbeats = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.probes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.probes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.inputDrops = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.inputDrops = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.lastCpu = null;
                        object.combining = null;
                        object.pool = null;
                        object.ring = null;
                        object.exercises = null;
                        object.observation = null;
                        object.timing = null;
                        object.latency = null;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.lostTimingEvents = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.lostTimingEvents = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.burstRemaining = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.burstRemaining = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.file = null;
                        object.lastSealed = null;
                    }
                    if (message.version != null && $Object.hasOwnProperty.call(message, "version"))
                        object.version = message.version;
                    if (message.status != null && $Object.hasOwnProperty.call(message, "status"))
                        object.status = message.status;
                    if (message.session != null && $Object.hasOwnProperty.call(message, "session"))
                        object.session = message.session;
                    if (message.error != null && $Object.hasOwnProperty.call(message, "error"))
                        object.error = message.error;
                    if (message.backend != null && $Object.hasOwnProperty.call(message, "backend"))
                        object.backend = message.backend;
                    if (message.direct != null && $Object.hasOwnProperty.call(message, "direct"))
                        object.direct = message.direct;
                    if (message.elapsedMs != null && $Object.hasOwnProperty.call(message, "elapsedMs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.elapsedMs = typeof message.elapsedMs === "number" ? $BigInt(message.elapsedMs) : $util.Long.fromBits(message.elapsedMs.low >>> 0, message.elapsedMs.high >>> 0, true).toBigInt();
                        else if (typeof message.elapsedMs === "number")
                            object.elapsedMs = options.longs === $String ? $String(message.elapsedMs) : message.elapsedMs;
                        else
                            object.elapsedMs = options.longs === $String ? $util.Long.prototype.toString.call(message.elapsedMs) : options.longs === $Number ? new $util.LongBits(message.elapsedMs.low >>> 0, message.elapsedMs.high >>> 0).toNumber(true) : message.elapsedMs;
                    if (message.cpu != null && $Object.hasOwnProperty.call(message, "cpu"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.cpu = typeof message.cpu === "number" ? $BigInt(message.cpu) : $util.Long.fromBits(message.cpu.low >>> 0, message.cpu.high >>> 0, true).toBigInt();
                        else if (typeof message.cpu === "number")
                            object.cpu = options.longs === $String ? $String(message.cpu) : message.cpu;
                        else
                            object.cpu = options.longs === $String ? $util.Long.prototype.toString.call(message.cpu) : options.longs === $Number ? new $util.LongBits(message.cpu.low >>> 0, message.cpu.high >>> 0).toNumber(true) : message.cpu;
                    if (message.heartbeats != null && $Object.hasOwnProperty.call(message, "heartbeats"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.heartbeats = typeof message.heartbeats === "number" ? $BigInt(message.heartbeats) : $util.Long.fromBits(message.heartbeats.low >>> 0, message.heartbeats.high >>> 0, true).toBigInt();
                        else if (typeof message.heartbeats === "number")
                            object.heartbeats = options.longs === $String ? $String(message.heartbeats) : message.heartbeats;
                        else
                            object.heartbeats = options.longs === $String ? $util.Long.prototype.toString.call(message.heartbeats) : options.longs === $Number ? new $util.LongBits(message.heartbeats.low >>> 0, message.heartbeats.high >>> 0).toNumber(true) : message.heartbeats;
                    if (message.probes != null && $Object.hasOwnProperty.call(message, "probes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.probes = typeof message.probes === "number" ? $BigInt(message.probes) : $util.Long.fromBits(message.probes.low >>> 0, message.probes.high >>> 0, true).toBigInt();
                        else if (typeof message.probes === "number")
                            object.probes = options.longs === $String ? $String(message.probes) : message.probes;
                        else
                            object.probes = options.longs === $String ? $util.Long.prototype.toString.call(message.probes) : options.longs === $Number ? new $util.LongBits(message.probes.low >>> 0, message.probes.high >>> 0).toNumber(true) : message.probes;
                    if (message.inputDrops != null && $Object.hasOwnProperty.call(message, "inputDrops"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.inputDrops = typeof message.inputDrops === "number" ? $BigInt(message.inputDrops) : $util.Long.fromBits(message.inputDrops.low >>> 0, message.inputDrops.high >>> 0, true).toBigInt();
                        else if (typeof message.inputDrops === "number")
                            object.inputDrops = options.longs === $String ? $String(message.inputDrops) : message.inputDrops;
                        else
                            object.inputDrops = options.longs === $String ? $util.Long.prototype.toString.call(message.inputDrops) : options.longs === $Number ? new $util.LongBits(message.inputDrops.low >>> 0, message.inputDrops.high >>> 0).toNumber(true) : message.inputDrops;
                    if (message.lastCpu != null && $Object.hasOwnProperty.call(message, "lastCpu"))
                        object.lastCpu = $root.doramcap.storage.v1.LastCpu.toObject(message.lastCpu, options, _depth + 1);
                    if (message.combining != null && $Object.hasOwnProperty.call(message, "combining"))
                        object.combining = $root.doramcap.storage.v1.Combining.toObject(message.combining, options, _depth + 1);
                    if (message.pool != null && $Object.hasOwnProperty.call(message, "pool"))
                        object.pool = $root.doramcap.storage.v1.Pool.toObject(message.pool, options, _depth + 1);
                    if (message.ring != null && $Object.hasOwnProperty.call(message, "ring"))
                        object.ring = $root.doramcap.storage.v1.Ring.toObject(message.ring, options, _depth + 1);
                    if (message.exercises != null && $Object.hasOwnProperty.call(message, "exercises"))
                        object.exercises = $root.doramcap.storage.v1.Exercises.toObject(message.exercises, options, _depth + 1);
                    if (message.observation != null && $Object.hasOwnProperty.call(message, "observation"))
                        object.observation = $root.doramcap.storage.v1.Observation.toObject(message.observation, options, _depth + 1);
                    if (message.activeWrites && message.activeWrites.length) {
                        object.activeWrites = $Array(message.activeWrites.length);
                        for (let j = 0; j < message.activeWrites.length; ++j)
                            object.activeWrites[j] = $root.doramcap.storage.v1.ActiveWrite.toObject(message.activeWrites[j], options, _depth + 1);
                    }
                    if (message.timing != null && $Object.hasOwnProperty.call(message, "timing"))
                        object.timing = $root.doramcap.storage.v1.Timing.toObject(message.timing, options, _depth + 1);
                    if (message.latency != null && $Object.hasOwnProperty.call(message, "latency"))
                        object.latency = $root.doramcap.storage.v1.Stats.toObject(message.latency, options, _depth + 1);
                    if (message.lostTimingEvents != null && $Object.hasOwnProperty.call(message, "lostTimingEvents"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.lostTimingEvents = typeof message.lostTimingEvents === "number" ? $BigInt(message.lostTimingEvents) : $util.Long.fromBits(message.lostTimingEvents.low >>> 0, message.lostTimingEvents.high >>> 0, true).toBigInt();
                        else if (typeof message.lostTimingEvents === "number")
                            object.lostTimingEvents = options.longs === $String ? $String(message.lostTimingEvents) : message.lostTimingEvents;
                        else
                            object.lostTimingEvents = options.longs === $String ? $util.Long.prototype.toString.call(message.lostTimingEvents) : options.longs === $Number ? new $util.LongBits(message.lostTimingEvents.low >>> 0, message.lostTimingEvents.high >>> 0).toNumber(true) : message.lostTimingEvents;
                    if (message.burstRemaining != null && $Object.hasOwnProperty.call(message, "burstRemaining"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.burstRemaining = typeof message.burstRemaining === "number" ? $BigInt(message.burstRemaining) : $util.Long.fromBits(message.burstRemaining.low >>> 0, message.burstRemaining.high >>> 0, true).toBigInt();
                        else if (typeof message.burstRemaining === "number")
                            object.burstRemaining = options.longs === $String ? $String(message.burstRemaining) : message.burstRemaining;
                        else
                            object.burstRemaining = options.longs === $String ? $util.Long.prototype.toString.call(message.burstRemaining) : options.longs === $Number ? new $util.LongBits(message.burstRemaining.low >>> 0, message.burstRemaining.high >>> 0).toNumber(true) : message.burstRemaining;
                    if (message.receipts && message.receipts.length) {
                        object.receipts = $Array(message.receipts.length);
                        for (let j = 0; j < message.receipts.length; ++j)
                            object.receipts[j] = $root.doramcap.storage.v1.Receipt.toObject(message.receipts[j], options, _depth + 1);
                    }
                    if (message.messages && message.messages.length) {
                        object.messages = $Array(message.messages.length);
                        for (let j = 0; j < message.messages.length; ++j)
                            object.messages[j] = $root.doramcap.storage.v1.Input.toObject(message.messages[j], options, _depth + 1);
                    }
                    if (message.events && message.events.length) {
                        object.events = $Array(message.events.length);
                        for (let j = 0; j < message.events.length; ++j)
                            object.events[j] = $root.doramcap.storage.v1.Event.toObject(message.events[j], options, _depth + 1);
                    }
                    if (message.file != null && $Object.hasOwnProperty.call(message, "file"))
                        object.file = $root.doramcap.storage.v1.File.toObject(message.file, options, _depth + 1);
                    if (message.lastSealed != null && $Object.hasOwnProperty.call(message, "lastSealed"))
                        object.lastSealed = $root.doramcap.storage.v1.File.toObject(message.lastSealed, options, _depth + 1);
                    return object;
                };

                /**
                 * Converts this Snapshot to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Snapshot
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Snapshot.prototype.toJSON = function() {
                    return Snapshot.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Snapshot
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Snapshot
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Snapshot.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Snapshot";
                };

                return Snapshot;
            })();

            v1.LastCpu = (function() {

                /**
                 * Properties of a LastCpu.
                 * @typedef {Object} doramcap.storage.v1.LastCpu.$Properties
                 * @property {number|Long|null} [sequence] LastCpu sequence
                 * @property {number|Long|null} [rows] LastCpu rows
                 * @property {number|Long|null} [dropped] LastCpu dropped
                 * @property {string|null} [source] LastCpu source
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a LastCpu.
                 * @memberof doramcap.storage.v1
                 * @interface ILastCpu
                 * @augments doramcap.storage.v1.LastCpu.$Properties
                 * @deprecated Use doramcap.storage.v1.LastCpu.$Properties instead.
                 */

                /**
                 * Shape of a LastCpu.
                 * @typedef {doramcap.storage.v1.LastCpu.$Properties} doramcap.storage.v1.LastCpu.$Shape
                 */

                /**
                 * Constructs a new LastCpu.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a LastCpu.
                 * @constructor
                 * @param {doramcap.storage.v1.LastCpu.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const LastCpu = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * LastCpu sequence.
                 * @member {number|Long} sequence
                 * @memberof doramcap.storage.v1.LastCpu
                 * @instance
                 */
                LastCpu.prototype.sequence = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * LastCpu rows.
                 * @member {number|Long} rows
                 * @memberof doramcap.storage.v1.LastCpu
                 * @instance
                 */
                LastCpu.prototype.rows = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * LastCpu dropped.
                 * @member {number|Long} dropped
                 * @memberof doramcap.storage.v1.LastCpu
                 * @instance
                 */
                LastCpu.prototype.dropped = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * LastCpu source.
                 * @member {string} source
                 * @memberof doramcap.storage.v1.LastCpu
                 * @instance
                 */
                LastCpu.prototype.source = "";

                /**
                 * Encodes the specified LastCpu message. Does not implicitly {@link doramcap.storage.v1.LastCpu.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.LastCpu
                 * @static
                 * @param {doramcap.storage.v1.LastCpu.$Properties} message LastCpu message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                LastCpu.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence") && (typeof message.sequence === "object" ? message.sequence.low || message.sequence.high : message.sequence !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.sequence);
                    if (message.rows != null && $Object.hasOwnProperty.call(message, "rows") && (typeof message.rows === "object" ? message.rows.low || message.rows.high : message.rows !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.rows);
                    if (message.dropped != null && $Object.hasOwnProperty.call(message, "dropped") && (typeof message.dropped === "object" ? message.dropped.low || message.dropped.high : message.dropped !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.dropped);
                    if (message.source != null && $Object.hasOwnProperty.call(message, "source") && message.source !== "")
                        writer.uint32(/* id 4, wireType 2 =*/34).string(message.source);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a LastCpu message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.LastCpu
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.LastCpu & doramcap.storage.v1.LastCpu.$Shape} LastCpu
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                LastCpu.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.LastCpu();
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
                                    message.sequence = value;
                                else
                                    delete message.sequence;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.rows = value;
                                else
                                    delete message.rows;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.dropped = value;
                                else
                                    delete message.dropped;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.source = value;
                                else
                                    delete message.source;
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
                 * Creates a LastCpu message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.LastCpu
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.LastCpu} LastCpu
                 */
                LastCpu.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.LastCpu)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.LastCpu: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.LastCpu();
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
                    if (object.rows != null)
                        if (typeof object.rows === "object" ? object.rows.low || object.rows.high : $Number(object.rows) !== 0)
                            if ($util.Long)
                                message.rows = $util.Long.fromValue(object.rows, true);
                            else if (typeof object.rows === "string")
                                message.rows = $parseInt(object.rows, 10);
                            else if (typeof object.rows === "number")
                                message.rows = object.rows;
                            else if (typeof object.rows === "object")
                                message.rows = new $util.LongBits(object.rows.low >>> 0, object.rows.high >>> 0).toNumber(true);
                    if (object.dropped != null)
                        if (typeof object.dropped === "object" ? object.dropped.low || object.dropped.high : $Number(object.dropped) !== 0)
                            if ($util.Long)
                                message.dropped = $util.Long.fromValue(object.dropped, true);
                            else if (typeof object.dropped === "string")
                                message.dropped = $parseInt(object.dropped, 10);
                            else if (typeof object.dropped === "number")
                                message.dropped = object.dropped;
                            else if (typeof object.dropped === "object")
                                message.dropped = new $util.LongBits(object.dropped.low >>> 0, object.dropped.high >>> 0).toNumber(true);
                    if (object.source != null)
                        if (typeof object.source !== "string" || object.source.length)
                            message.source = $String(object.source);
                    return message;
                };

                /**
                 * Creates a plain object from a LastCpu message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.LastCpu
                 * @static
                 * @param {doramcap.storage.v1.LastCpu} message LastCpu
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                LastCpu.toObject = function (message, options, _depth) {
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
                            object.sequence = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.sequence = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.rows = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.rows = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.dropped = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.dropped = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.source = "";
                    }
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.sequence = typeof message.sequence === "number" ? $BigInt(message.sequence) : $util.Long.fromBits(message.sequence.low >>> 0, message.sequence.high >>> 0, true).toBigInt();
                        else if (typeof message.sequence === "number")
                            object.sequence = options.longs === $String ? $String(message.sequence) : message.sequence;
                        else
                            object.sequence = options.longs === $String ? $util.Long.prototype.toString.call(message.sequence) : options.longs === $Number ? new $util.LongBits(message.sequence.low >>> 0, message.sequence.high >>> 0).toNumber(true) : message.sequence;
                    if (message.rows != null && $Object.hasOwnProperty.call(message, "rows"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.rows = typeof message.rows === "number" ? $BigInt(message.rows) : $util.Long.fromBits(message.rows.low >>> 0, message.rows.high >>> 0, true).toBigInt();
                        else if (typeof message.rows === "number")
                            object.rows = options.longs === $String ? $String(message.rows) : message.rows;
                        else
                            object.rows = options.longs === $String ? $util.Long.prototype.toString.call(message.rows) : options.longs === $Number ? new $util.LongBits(message.rows.low >>> 0, message.rows.high >>> 0).toNumber(true) : message.rows;
                    if (message.dropped != null && $Object.hasOwnProperty.call(message, "dropped"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.dropped = typeof message.dropped === "number" ? $BigInt(message.dropped) : $util.Long.fromBits(message.dropped.low >>> 0, message.dropped.high >>> 0, true).toBigInt();
                        else if (typeof message.dropped === "number")
                            object.dropped = options.longs === $String ? $String(message.dropped) : message.dropped;
                        else
                            object.dropped = options.longs === $String ? $util.Long.prototype.toString.call(message.dropped) : options.longs === $Number ? new $util.LongBits(message.dropped.low >>> 0, message.dropped.high >>> 0).toNumber(true) : message.dropped;
                    if (message.source != null && $Object.hasOwnProperty.call(message, "source"))
                        object.source = message.source;
                    return object;
                };

                /**
                 * Converts this LastCpu to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.LastCpu
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                LastCpu.prototype.toJSON = function() {
                    return LastCpu.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for LastCpu
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.LastCpu
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                LastCpu.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.LastCpu";
                };

                return LastCpu;
            })();

            v1.Combining = (function() {

                /**
                 * Properties of a Combining.
                 * @typedef {Object} doramcap.storage.v1.Combining.$Properties
                 * @property {number|Long|null} [used] Combining used
                 * @property {number|Long|null} [messages] Combining messages
                 * @property {number|Long|null} [capacity] Combining capacity
                 * @property {number|null} [ageMs] Combining ageMs
                 * @property {number|Long|null} [deadlineMs] Combining deadlineMs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Combining.
                 * @memberof doramcap.storage.v1
                 * @interface ICombining
                 * @augments doramcap.storage.v1.Combining.$Properties
                 * @deprecated Use doramcap.storage.v1.Combining.$Properties instead.
                 */

                /**
                 * Shape of a Combining.
                 * @typedef {doramcap.storage.v1.Combining.$Properties} doramcap.storage.v1.Combining.$Shape
                 */

                /**
                 * Constructs a new Combining.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a Combining.
                 * @constructor
                 * @param {doramcap.storage.v1.Combining.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Combining = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Combining used.
                 * @member {number|Long} used
                 * @memberof doramcap.storage.v1.Combining
                 * @instance
                 */
                Combining.prototype.used = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Combining messages.
                 * @member {number|Long} messages
                 * @memberof doramcap.storage.v1.Combining
                 * @instance
                 */
                Combining.prototype.messages = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Combining capacity.
                 * @member {number|Long} capacity
                 * @memberof doramcap.storage.v1.Combining
                 * @instance
                 */
                Combining.prototype.capacity = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Combining ageMs.
                 * @member {number|null|undefined} ageMs
                 * @memberof doramcap.storage.v1.Combining
                 * @instance
                 */
                Combining.prototype.ageMs = null;

                /**
                 * Combining deadlineMs.
                 * @member {number|Long} deadlineMs
                 * @memberof doramcap.storage.v1.Combining
                 * @instance
                 */
                Combining.prototype.deadlineMs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Combining.prototype, "_ageMs", {
                    get: $util.oneOfGetter($oneOfFields = ["ageMs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Combining message. Does not implicitly {@link doramcap.storage.v1.Combining.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Combining
                 * @static
                 * @param {doramcap.storage.v1.Combining.$Properties} message Combining message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Combining.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.used != null && $Object.hasOwnProperty.call(message, "used") && (typeof message.used === "object" ? message.used.low || message.used.high : message.used !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.used);
                    if (message.messages != null && $Object.hasOwnProperty.call(message, "messages") && (typeof message.messages === "object" ? message.messages.low || message.messages.high : message.messages !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.messages);
                    if (message.capacity != null && $Object.hasOwnProperty.call(message, "capacity") && (typeof message.capacity === "object" ? message.capacity.low || message.capacity.high : message.capacity !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.capacity);
                    if (message.ageMs != null && $Object.hasOwnProperty.call(message, "ageMs"))
                        writer.uint32(/* id 4, wireType 1 =*/33).double(message.ageMs);
                    if (message.deadlineMs != null && $Object.hasOwnProperty.call(message, "deadlineMs") && (typeof message.deadlineMs === "object" ? message.deadlineMs.low || message.deadlineMs.high : message.deadlineMs !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.deadlineMs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Combining message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Combining
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Combining & doramcap.storage.v1.Combining.$Shape} Combining
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Combining.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Combining();
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
                                    message.used = value;
                                else
                                    delete message.used;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.messages = value;
                                else
                                    delete message.messages;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.capacity = value;
                                else
                                    delete message.capacity;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 1)
                                    break;
                                message.ageMs = reader.double();
                                message._ageMs = "ageMs";
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.deadlineMs = value;
                                else
                                    delete message.deadlineMs;
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
                 * Creates a Combining message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Combining
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Combining} Combining
                 */
                Combining.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Combining)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Combining: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Combining();
                    if (object.used != null)
                        if (typeof object.used === "object" ? object.used.low || object.used.high : $Number(object.used) !== 0)
                            if ($util.Long)
                                message.used = $util.Long.fromValue(object.used, true);
                            else if (typeof object.used === "string")
                                message.used = $parseInt(object.used, 10);
                            else if (typeof object.used === "number")
                                message.used = object.used;
                            else if (typeof object.used === "object")
                                message.used = new $util.LongBits(object.used.low >>> 0, object.used.high >>> 0).toNumber(true);
                    if (object.messages != null)
                        if (typeof object.messages === "object" ? object.messages.low || object.messages.high : $Number(object.messages) !== 0)
                            if ($util.Long)
                                message.messages = $util.Long.fromValue(object.messages, true);
                            else if (typeof object.messages === "string")
                                message.messages = $parseInt(object.messages, 10);
                            else if (typeof object.messages === "number")
                                message.messages = object.messages;
                            else if (typeof object.messages === "object")
                                message.messages = new $util.LongBits(object.messages.low >>> 0, object.messages.high >>> 0).toNumber(true);
                    if (object.capacity != null)
                        if (typeof object.capacity === "object" ? object.capacity.low || object.capacity.high : $Number(object.capacity) !== 0)
                            if ($util.Long)
                                message.capacity = $util.Long.fromValue(object.capacity, true);
                            else if (typeof object.capacity === "string")
                                message.capacity = $parseInt(object.capacity, 10);
                            else if (typeof object.capacity === "number")
                                message.capacity = object.capacity;
                            else if (typeof object.capacity === "object")
                                message.capacity = new $util.LongBits(object.capacity.low >>> 0, object.capacity.high >>> 0).toNumber(true);
                    if (object.ageMs != null)
                        message.ageMs = $Number(object.ageMs);
                    if (object.deadlineMs != null)
                        if (typeof object.deadlineMs === "object" ? object.deadlineMs.low || object.deadlineMs.high : $Number(object.deadlineMs) !== 0)
                            if ($util.Long)
                                message.deadlineMs = $util.Long.fromValue(object.deadlineMs, true);
                            else if (typeof object.deadlineMs === "string")
                                message.deadlineMs = $parseInt(object.deadlineMs, 10);
                            else if (typeof object.deadlineMs === "number")
                                message.deadlineMs = object.deadlineMs;
                            else if (typeof object.deadlineMs === "object")
                                message.deadlineMs = new $util.LongBits(object.deadlineMs.low >>> 0, object.deadlineMs.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a Combining message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Combining
                 * @static
                 * @param {doramcap.storage.v1.Combining} message Combining
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Combining.toObject = function (message, options, _depth) {
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
                            object.used = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.used = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.messages = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.messages = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.capacity = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.capacity = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.deadlineMs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.deadlineMs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.used != null && $Object.hasOwnProperty.call(message, "used"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.used = typeof message.used === "number" ? $BigInt(message.used) : $util.Long.fromBits(message.used.low >>> 0, message.used.high >>> 0, true).toBigInt();
                        else if (typeof message.used === "number")
                            object.used = options.longs === $String ? $String(message.used) : message.used;
                        else
                            object.used = options.longs === $String ? $util.Long.prototype.toString.call(message.used) : options.longs === $Number ? new $util.LongBits(message.used.low >>> 0, message.used.high >>> 0).toNumber(true) : message.used;
                    if (message.messages != null && $Object.hasOwnProperty.call(message, "messages"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.messages = typeof message.messages === "number" ? $BigInt(message.messages) : $util.Long.fromBits(message.messages.low >>> 0, message.messages.high >>> 0, true).toBigInt();
                        else if (typeof message.messages === "number")
                            object.messages = options.longs === $String ? $String(message.messages) : message.messages;
                        else
                            object.messages = options.longs === $String ? $util.Long.prototype.toString.call(message.messages) : options.longs === $Number ? new $util.LongBits(message.messages.low >>> 0, message.messages.high >>> 0).toNumber(true) : message.messages;
                    if (message.capacity != null && $Object.hasOwnProperty.call(message, "capacity"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.capacity = typeof message.capacity === "number" ? $BigInt(message.capacity) : $util.Long.fromBits(message.capacity.low >>> 0, message.capacity.high >>> 0, true).toBigInt();
                        else if (typeof message.capacity === "number")
                            object.capacity = options.longs === $String ? $String(message.capacity) : message.capacity;
                        else
                            object.capacity = options.longs === $String ? $util.Long.prototype.toString.call(message.capacity) : options.longs === $Number ? new $util.LongBits(message.capacity.low >>> 0, message.capacity.high >>> 0).toNumber(true) : message.capacity;
                    if (message.ageMs != null && $Object.hasOwnProperty.call(message, "ageMs"))
                        object.ageMs = options.json && !$isFinite(message.ageMs) ? $String(message.ageMs) : message.ageMs;
                    if (message.deadlineMs != null && $Object.hasOwnProperty.call(message, "deadlineMs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.deadlineMs = typeof message.deadlineMs === "number" ? $BigInt(message.deadlineMs) : $util.Long.fromBits(message.deadlineMs.low >>> 0, message.deadlineMs.high >>> 0, true).toBigInt();
                        else if (typeof message.deadlineMs === "number")
                            object.deadlineMs = options.longs === $String ? $String(message.deadlineMs) : message.deadlineMs;
                        else
                            object.deadlineMs = options.longs === $String ? $util.Long.prototype.toString.call(message.deadlineMs) : options.longs === $Number ? new $util.LongBits(message.deadlineMs.low >>> 0, message.deadlineMs.high >>> 0).toNumber(true) : message.deadlineMs;
                    return object;
                };

                /**
                 * Converts this Combining to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Combining
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Combining.prototype.toJSON = function() {
                    return Combining.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Combining
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Combining
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Combining.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Combining";
                };

                return Combining;
            })();

            v1.Pool = (function() {

                /**
                 * Properties of a Pool.
                 * @typedef {Object} doramcap.storage.v1.Pool.$Properties
                 * @property {number|Long|null} [free] Pool free
                 * @property {number|Long|null} [total] Pool total
                 * @property {number|Long|null} [allocatedBytes] Pool allocatedBytes
                 * @property {number|Long|null} [freeBytes] Pool freeBytes
                 * @property {number|Long|null} [inFlightBytes] Pool inFlightBytes
                 * @property {number|Long|null} [registeredBytes] Pool registeredBytes
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Pool.
                 * @memberof doramcap.storage.v1
                 * @interface IPool
                 * @augments doramcap.storage.v1.Pool.$Properties
                 * @deprecated Use doramcap.storage.v1.Pool.$Properties instead.
                 */

                /**
                 * Shape of a Pool.
                 * @typedef {doramcap.storage.v1.Pool.$Properties} doramcap.storage.v1.Pool.$Shape
                 */

                /**
                 * Constructs a new Pool.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a Pool.
                 * @constructor
                 * @param {doramcap.storage.v1.Pool.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Pool = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Pool free.
                 * @member {number|Long} free
                 * @memberof doramcap.storage.v1.Pool
                 * @instance
                 */
                Pool.prototype.free = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Pool total.
                 * @member {number|Long} total
                 * @memberof doramcap.storage.v1.Pool
                 * @instance
                 */
                Pool.prototype.total = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Pool allocatedBytes.
                 * @member {number|Long} allocatedBytes
                 * @memberof doramcap.storage.v1.Pool
                 * @instance
                 */
                Pool.prototype.allocatedBytes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Pool freeBytes.
                 * @member {number|Long} freeBytes
                 * @memberof doramcap.storage.v1.Pool
                 * @instance
                 */
                Pool.prototype.freeBytes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Pool inFlightBytes.
                 * @member {number|Long} inFlightBytes
                 * @memberof doramcap.storage.v1.Pool
                 * @instance
                 */
                Pool.prototype.inFlightBytes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Pool registeredBytes.
                 * @member {number|Long} registeredBytes
                 * @memberof doramcap.storage.v1.Pool
                 * @instance
                 */
                Pool.prototype.registeredBytes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified Pool message. Does not implicitly {@link doramcap.storage.v1.Pool.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Pool
                 * @static
                 * @param {doramcap.storage.v1.Pool.$Properties} message Pool message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Pool.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.free != null && $Object.hasOwnProperty.call(message, "free") && (typeof message.free === "object" ? message.free.low || message.free.high : message.free !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.free);
                    if (message.total != null && $Object.hasOwnProperty.call(message, "total") && (typeof message.total === "object" ? message.total.low || message.total.high : message.total !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.total);
                    if (message.allocatedBytes != null && $Object.hasOwnProperty.call(message, "allocatedBytes") && (typeof message.allocatedBytes === "object" ? message.allocatedBytes.low || message.allocatedBytes.high : message.allocatedBytes !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.allocatedBytes);
                    if (message.freeBytes != null && $Object.hasOwnProperty.call(message, "freeBytes") && (typeof message.freeBytes === "object" ? message.freeBytes.low || message.freeBytes.high : message.freeBytes !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.freeBytes);
                    if (message.inFlightBytes != null && $Object.hasOwnProperty.call(message, "inFlightBytes") && (typeof message.inFlightBytes === "object" ? message.inFlightBytes.low || message.inFlightBytes.high : message.inFlightBytes !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.inFlightBytes);
                    if (message.registeredBytes != null && $Object.hasOwnProperty.call(message, "registeredBytes") && (typeof message.registeredBytes === "object" ? message.registeredBytes.low || message.registeredBytes.high : message.registeredBytes !== 0))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.registeredBytes);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Pool message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Pool
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Pool & doramcap.storage.v1.Pool.$Shape} Pool
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Pool.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Pool();
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
                                    message.free = value;
                                else
                                    delete message.free;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.total = value;
                                else
                                    delete message.total;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.allocatedBytes = value;
                                else
                                    delete message.allocatedBytes;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.freeBytes = value;
                                else
                                    delete message.freeBytes;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.inFlightBytes = value;
                                else
                                    delete message.inFlightBytes;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.registeredBytes = value;
                                else
                                    delete message.registeredBytes;
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
                 * Creates a Pool message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Pool
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Pool} Pool
                 */
                Pool.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Pool)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Pool: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Pool();
                    if (object.free != null)
                        if (typeof object.free === "object" ? object.free.low || object.free.high : $Number(object.free) !== 0)
                            if ($util.Long)
                                message.free = $util.Long.fromValue(object.free, true);
                            else if (typeof object.free === "string")
                                message.free = $parseInt(object.free, 10);
                            else if (typeof object.free === "number")
                                message.free = object.free;
                            else if (typeof object.free === "object")
                                message.free = new $util.LongBits(object.free.low >>> 0, object.free.high >>> 0).toNumber(true);
                    if (object.total != null)
                        if (typeof object.total === "object" ? object.total.low || object.total.high : $Number(object.total) !== 0)
                            if ($util.Long)
                                message.total = $util.Long.fromValue(object.total, true);
                            else if (typeof object.total === "string")
                                message.total = $parseInt(object.total, 10);
                            else if (typeof object.total === "number")
                                message.total = object.total;
                            else if (typeof object.total === "object")
                                message.total = new $util.LongBits(object.total.low >>> 0, object.total.high >>> 0).toNumber(true);
                    if (object.allocatedBytes != null)
                        if (typeof object.allocatedBytes === "object" ? object.allocatedBytes.low || object.allocatedBytes.high : $Number(object.allocatedBytes) !== 0)
                            if ($util.Long)
                                message.allocatedBytes = $util.Long.fromValue(object.allocatedBytes, true);
                            else if (typeof object.allocatedBytes === "string")
                                message.allocatedBytes = $parseInt(object.allocatedBytes, 10);
                            else if (typeof object.allocatedBytes === "number")
                                message.allocatedBytes = object.allocatedBytes;
                            else if (typeof object.allocatedBytes === "object")
                                message.allocatedBytes = new $util.LongBits(object.allocatedBytes.low >>> 0, object.allocatedBytes.high >>> 0).toNumber(true);
                    if (object.freeBytes != null)
                        if (typeof object.freeBytes === "object" ? object.freeBytes.low || object.freeBytes.high : $Number(object.freeBytes) !== 0)
                            if ($util.Long)
                                message.freeBytes = $util.Long.fromValue(object.freeBytes, true);
                            else if (typeof object.freeBytes === "string")
                                message.freeBytes = $parseInt(object.freeBytes, 10);
                            else if (typeof object.freeBytes === "number")
                                message.freeBytes = object.freeBytes;
                            else if (typeof object.freeBytes === "object")
                                message.freeBytes = new $util.LongBits(object.freeBytes.low >>> 0, object.freeBytes.high >>> 0).toNumber(true);
                    if (object.inFlightBytes != null)
                        if (typeof object.inFlightBytes === "object" ? object.inFlightBytes.low || object.inFlightBytes.high : $Number(object.inFlightBytes) !== 0)
                            if ($util.Long)
                                message.inFlightBytes = $util.Long.fromValue(object.inFlightBytes, true);
                            else if (typeof object.inFlightBytes === "string")
                                message.inFlightBytes = $parseInt(object.inFlightBytes, 10);
                            else if (typeof object.inFlightBytes === "number")
                                message.inFlightBytes = object.inFlightBytes;
                            else if (typeof object.inFlightBytes === "object")
                                message.inFlightBytes = new $util.LongBits(object.inFlightBytes.low >>> 0, object.inFlightBytes.high >>> 0).toNumber(true);
                    if (object.registeredBytes != null)
                        if (typeof object.registeredBytes === "object" ? object.registeredBytes.low || object.registeredBytes.high : $Number(object.registeredBytes) !== 0)
                            if ($util.Long)
                                message.registeredBytes = $util.Long.fromValue(object.registeredBytes, true);
                            else if (typeof object.registeredBytes === "string")
                                message.registeredBytes = $parseInt(object.registeredBytes, 10);
                            else if (typeof object.registeredBytes === "number")
                                message.registeredBytes = object.registeredBytes;
                            else if (typeof object.registeredBytes === "object")
                                message.registeredBytes = new $util.LongBits(object.registeredBytes.low >>> 0, object.registeredBytes.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a Pool message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Pool
                 * @static
                 * @param {doramcap.storage.v1.Pool} message Pool
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Pool.toObject = function (message, options, _depth) {
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
                            object.free = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.free = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.total = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.total = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.allocatedBytes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.allocatedBytes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.freeBytes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.freeBytes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.inFlightBytes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.inFlightBytes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.registeredBytes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.registeredBytes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.free != null && $Object.hasOwnProperty.call(message, "free"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.free = typeof message.free === "number" ? $BigInt(message.free) : $util.Long.fromBits(message.free.low >>> 0, message.free.high >>> 0, true).toBigInt();
                        else if (typeof message.free === "number")
                            object.free = options.longs === $String ? $String(message.free) : message.free;
                        else
                            object.free = options.longs === $String ? $util.Long.prototype.toString.call(message.free) : options.longs === $Number ? new $util.LongBits(message.free.low >>> 0, message.free.high >>> 0).toNumber(true) : message.free;
                    if (message.total != null && $Object.hasOwnProperty.call(message, "total"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.total = typeof message.total === "number" ? $BigInt(message.total) : $util.Long.fromBits(message.total.low >>> 0, message.total.high >>> 0, true).toBigInt();
                        else if (typeof message.total === "number")
                            object.total = options.longs === $String ? $String(message.total) : message.total;
                        else
                            object.total = options.longs === $String ? $util.Long.prototype.toString.call(message.total) : options.longs === $Number ? new $util.LongBits(message.total.low >>> 0, message.total.high >>> 0).toNumber(true) : message.total;
                    if (message.allocatedBytes != null && $Object.hasOwnProperty.call(message, "allocatedBytes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.allocatedBytes = typeof message.allocatedBytes === "number" ? $BigInt(message.allocatedBytes) : $util.Long.fromBits(message.allocatedBytes.low >>> 0, message.allocatedBytes.high >>> 0, true).toBigInt();
                        else if (typeof message.allocatedBytes === "number")
                            object.allocatedBytes = options.longs === $String ? $String(message.allocatedBytes) : message.allocatedBytes;
                        else
                            object.allocatedBytes = options.longs === $String ? $util.Long.prototype.toString.call(message.allocatedBytes) : options.longs === $Number ? new $util.LongBits(message.allocatedBytes.low >>> 0, message.allocatedBytes.high >>> 0).toNumber(true) : message.allocatedBytes;
                    if (message.freeBytes != null && $Object.hasOwnProperty.call(message, "freeBytes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.freeBytes = typeof message.freeBytes === "number" ? $BigInt(message.freeBytes) : $util.Long.fromBits(message.freeBytes.low >>> 0, message.freeBytes.high >>> 0, true).toBigInt();
                        else if (typeof message.freeBytes === "number")
                            object.freeBytes = options.longs === $String ? $String(message.freeBytes) : message.freeBytes;
                        else
                            object.freeBytes = options.longs === $String ? $util.Long.prototype.toString.call(message.freeBytes) : options.longs === $Number ? new $util.LongBits(message.freeBytes.low >>> 0, message.freeBytes.high >>> 0).toNumber(true) : message.freeBytes;
                    if (message.inFlightBytes != null && $Object.hasOwnProperty.call(message, "inFlightBytes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.inFlightBytes = typeof message.inFlightBytes === "number" ? $BigInt(message.inFlightBytes) : $util.Long.fromBits(message.inFlightBytes.low >>> 0, message.inFlightBytes.high >>> 0, true).toBigInt();
                        else if (typeof message.inFlightBytes === "number")
                            object.inFlightBytes = options.longs === $String ? $String(message.inFlightBytes) : message.inFlightBytes;
                        else
                            object.inFlightBytes = options.longs === $String ? $util.Long.prototype.toString.call(message.inFlightBytes) : options.longs === $Number ? new $util.LongBits(message.inFlightBytes.low >>> 0, message.inFlightBytes.high >>> 0).toNumber(true) : message.inFlightBytes;
                    if (message.registeredBytes != null && $Object.hasOwnProperty.call(message, "registeredBytes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.registeredBytes = typeof message.registeredBytes === "number" ? $BigInt(message.registeredBytes) : $util.Long.fromBits(message.registeredBytes.low >>> 0, message.registeredBytes.high >>> 0, true).toBigInt();
                        else if (typeof message.registeredBytes === "number")
                            object.registeredBytes = options.longs === $String ? $String(message.registeredBytes) : message.registeredBytes;
                        else
                            object.registeredBytes = options.longs === $String ? $util.Long.prototype.toString.call(message.registeredBytes) : options.longs === $Number ? new $util.LongBits(message.registeredBytes.low >>> 0, message.registeredBytes.high >>> 0).toNumber(true) : message.registeredBytes;
                    return object;
                };

                /**
                 * Converts this Pool to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Pool
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Pool.prototype.toJSON = function() {
                    return Pool.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Pool
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Pool
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Pool.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Pool";
                };

                return Pool;
            })();

            v1.Ring = (function() {

                /**
                 * Properties of a Ring.
                 * @typedef {Object} doramcap.storage.v1.Ring.$Properties
                 * @property {number|Long|null} [writer] Ring writer
                 * @property {number|Long|null} [file] Ring file
                 * @property {number|Long|null} [pendingCapacity] Ring pendingCapacity
                 * @property {number|Long|null} [outstanding] Ring outstanding
                 * @property {number|Long|null} [submitted] Ring submitted
                 * @property {number|Long|null} [completions] Ring completions
                 * @property {number|Long|null} [accepted] Ring accepted
                 * @property {number|Long|null} [completed] Ring completed
                 * @property {number|Long|null} [synced] Ring synced
                 * @property {number|Long|null} [awaitingSubmission] Ring awaitingSubmission
                 * @property {number|Long|null} [awaitingCqe] Ring awaitingCqe
                 * @property {number|Long|null} [pendingBytes] Ring pendingBytes
                 * @property {number|Long|null} [oldestUs] Ring oldestUs
                 * @property {number|Long|null} [registeredRegions] Ring registeredRegions
                 * @property {number|Long|null} [registrationUpdates] Ring registrationUpdates
                 * @property {number|Long|null} [acceptedWrites] Ring acceptedWrites
                 * @property {number|Long|null} [written] Ring written
                 * @property {number|Long|null} [failed] Ring failed
                 * @property {number|Long|null} [cancelled] Ring cancelled
                 * @property {number|Long|null} [rejectedCapacity] Ring rejectedCapacity
                 * @property {number|Long|null} [rejectedInvalid] Ring rejectedInvalid
                 * @property {number|Long|null} [rejectedClosed] Ring rejectedClosed
                 * @property {number|Long|null} [rejectedOther] Ring rejectedOther
                 * @property {number|Long|null} [retries] Ring retries
                 * @property {number|Long|null} [registrationHits] Ring registrationHits
                 * @property {number|Long|null} [registrationMisses] Ring registrationMisses
                 * @property {number|Long|null} [registrationFailures] Ring registrationFailures
                 * @property {number|Long|null} [syncFailures] Ring syncFailures
                 * @property {number|Long|null} [submitFailures] Ring submitFailures
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Ring.
                 * @memberof doramcap.storage.v1
                 * @interface IRing
                 * @augments doramcap.storage.v1.Ring.$Properties
                 * @deprecated Use doramcap.storage.v1.Ring.$Properties instead.
                 */

                /**
                 * Shape of a Ring.
                 * @typedef {doramcap.storage.v1.Ring.$Properties} doramcap.storage.v1.Ring.$Shape
                 */

                /**
                 * Constructs a new Ring.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a Ring.
                 * @constructor
                 * @param {doramcap.storage.v1.Ring.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Ring = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Ring writer.
                 * @member {number|Long} writer
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.writer = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring file.
                 * @member {number|Long} file
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.file = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring pendingCapacity.
                 * @member {number|Long} pendingCapacity
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.pendingCapacity = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring outstanding.
                 * @member {number|Long} outstanding
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.outstanding = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring submitted.
                 * @member {number|Long} submitted
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.submitted = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring completions.
                 * @member {number|Long} completions
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.completions = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring accepted.
                 * @member {number|Long} accepted
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.accepted = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring completed.
                 * @member {number|Long} completed
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.completed = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring synced.
                 * @member {number|Long} synced
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.synced = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring awaitingSubmission.
                 * @member {number|Long} awaitingSubmission
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.awaitingSubmission = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring awaitingCqe.
                 * @member {number|Long} awaitingCqe
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.awaitingCqe = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring pendingBytes.
                 * @member {number|Long} pendingBytes
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.pendingBytes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring oldestUs.
                 * @member {number|Long|null|undefined} oldestUs
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.oldestUs = null;

                /**
                 * Ring registeredRegions.
                 * @member {number|Long} registeredRegions
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.registeredRegions = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring registrationUpdates.
                 * @member {number|Long} registrationUpdates
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.registrationUpdates = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring acceptedWrites.
                 * @member {number|Long} acceptedWrites
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.acceptedWrites = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring written.
                 * @member {number|Long} written
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.written = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring failed.
                 * @member {number|Long} failed
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.failed = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring cancelled.
                 * @member {number|Long} cancelled
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.cancelled = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring rejectedCapacity.
                 * @member {number|Long} rejectedCapacity
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.rejectedCapacity = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring rejectedInvalid.
                 * @member {number|Long} rejectedInvalid
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.rejectedInvalid = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring rejectedClosed.
                 * @member {number|Long} rejectedClosed
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.rejectedClosed = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring rejectedOther.
                 * @member {number|Long} rejectedOther
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.rejectedOther = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring retries.
                 * @member {number|Long} retries
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.retries = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring registrationHits.
                 * @member {number|Long} registrationHits
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.registrationHits = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring registrationMisses.
                 * @member {number|Long} registrationMisses
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.registrationMisses = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring registrationFailures.
                 * @member {number|Long} registrationFailures
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.registrationFailures = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring syncFailures.
                 * @member {number|Long} syncFailures
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.syncFailures = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Ring submitFailures.
                 * @member {number|Long} submitFailures
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 */
                Ring.prototype.submitFailures = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Ring.prototype, "_oldestUs", {
                    get: $util.oneOfGetter($oneOfFields = ["oldestUs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Ring message. Does not implicitly {@link doramcap.storage.v1.Ring.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Ring
                 * @static
                 * @param {doramcap.storage.v1.Ring.$Properties} message Ring message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Ring.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.writer != null && $Object.hasOwnProperty.call(message, "writer") && (typeof message.writer === "object" ? message.writer.low || message.writer.high : message.writer !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.writer);
                    if (message.file != null && $Object.hasOwnProperty.call(message, "file") && (typeof message.file === "object" ? message.file.low || message.file.high : message.file !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.file);
                    if (message.pendingCapacity != null && $Object.hasOwnProperty.call(message, "pendingCapacity") && (typeof message.pendingCapacity === "object" ? message.pendingCapacity.low || message.pendingCapacity.high : message.pendingCapacity !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.pendingCapacity);
                    if (message.outstanding != null && $Object.hasOwnProperty.call(message, "outstanding") && (typeof message.outstanding === "object" ? message.outstanding.low || message.outstanding.high : message.outstanding !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.outstanding);
                    if (message.submitted != null && $Object.hasOwnProperty.call(message, "submitted") && (typeof message.submitted === "object" ? message.submitted.low || message.submitted.high : message.submitted !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.submitted);
                    if (message.completions != null && $Object.hasOwnProperty.call(message, "completions") && (typeof message.completions === "object" ? message.completions.low || message.completions.high : message.completions !== 0))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.completions);
                    if (message.accepted != null && $Object.hasOwnProperty.call(message, "accepted") && (typeof message.accepted === "object" ? message.accepted.low || message.accepted.high : message.accepted !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.accepted);
                    if (message.completed != null && $Object.hasOwnProperty.call(message, "completed") && (typeof message.completed === "object" ? message.completed.low || message.completed.high : message.completed !== 0))
                        writer.uint32(/* id 8, wireType 0 =*/64).uint64(message.completed);
                    if (message.synced != null && $Object.hasOwnProperty.call(message, "synced") && (typeof message.synced === "object" ? message.synced.low || message.synced.high : message.synced !== 0))
                        writer.uint32(/* id 9, wireType 0 =*/72).uint64(message.synced);
                    if (message.awaitingSubmission != null && $Object.hasOwnProperty.call(message, "awaitingSubmission") && (typeof message.awaitingSubmission === "object" ? message.awaitingSubmission.low || message.awaitingSubmission.high : message.awaitingSubmission !== 0))
                        writer.uint32(/* id 10, wireType 0 =*/80).uint64(message.awaitingSubmission);
                    if (message.awaitingCqe != null && $Object.hasOwnProperty.call(message, "awaitingCqe") && (typeof message.awaitingCqe === "object" ? message.awaitingCqe.low || message.awaitingCqe.high : message.awaitingCqe !== 0))
                        writer.uint32(/* id 11, wireType 0 =*/88).uint64(message.awaitingCqe);
                    if (message.pendingBytes != null && $Object.hasOwnProperty.call(message, "pendingBytes") && (typeof message.pendingBytes === "object" ? message.pendingBytes.low || message.pendingBytes.high : message.pendingBytes !== 0))
                        writer.uint32(/* id 12, wireType 0 =*/96).uint64(message.pendingBytes);
                    if (message.oldestUs != null && $Object.hasOwnProperty.call(message, "oldestUs"))
                        writer.uint32(/* id 13, wireType 0 =*/104).uint64(message.oldestUs);
                    if (message.registeredRegions != null && $Object.hasOwnProperty.call(message, "registeredRegions") && (typeof message.registeredRegions === "object" ? message.registeredRegions.low || message.registeredRegions.high : message.registeredRegions !== 0))
                        writer.uint32(/* id 14, wireType 0 =*/112).uint64(message.registeredRegions);
                    if (message.registrationUpdates != null && $Object.hasOwnProperty.call(message, "registrationUpdates") && (typeof message.registrationUpdates === "object" ? message.registrationUpdates.low || message.registrationUpdates.high : message.registrationUpdates !== 0))
                        writer.uint32(/* id 15, wireType 0 =*/120).uint64(message.registrationUpdates);
                    if (message.acceptedWrites != null && $Object.hasOwnProperty.call(message, "acceptedWrites") && (typeof message.acceptedWrites === "object" ? message.acceptedWrites.low || message.acceptedWrites.high : message.acceptedWrites !== 0))
                        writer.uint32(/* id 16, wireType 0 =*/128).uint64(message.acceptedWrites);
                    if (message.written != null && $Object.hasOwnProperty.call(message, "written") && (typeof message.written === "object" ? message.written.low || message.written.high : message.written !== 0))
                        writer.uint32(/* id 17, wireType 0 =*/136).uint64(message.written);
                    if (message.failed != null && $Object.hasOwnProperty.call(message, "failed") && (typeof message.failed === "object" ? message.failed.low || message.failed.high : message.failed !== 0))
                        writer.uint32(/* id 18, wireType 0 =*/144).uint64(message.failed);
                    if (message.cancelled != null && $Object.hasOwnProperty.call(message, "cancelled") && (typeof message.cancelled === "object" ? message.cancelled.low || message.cancelled.high : message.cancelled !== 0))
                        writer.uint32(/* id 19, wireType 0 =*/152).uint64(message.cancelled);
                    if (message.rejectedCapacity != null && $Object.hasOwnProperty.call(message, "rejectedCapacity") && (typeof message.rejectedCapacity === "object" ? message.rejectedCapacity.low || message.rejectedCapacity.high : message.rejectedCapacity !== 0))
                        writer.uint32(/* id 20, wireType 0 =*/160).uint64(message.rejectedCapacity);
                    if (message.rejectedInvalid != null && $Object.hasOwnProperty.call(message, "rejectedInvalid") && (typeof message.rejectedInvalid === "object" ? message.rejectedInvalid.low || message.rejectedInvalid.high : message.rejectedInvalid !== 0))
                        writer.uint32(/* id 21, wireType 0 =*/168).uint64(message.rejectedInvalid);
                    if (message.rejectedClosed != null && $Object.hasOwnProperty.call(message, "rejectedClosed") && (typeof message.rejectedClosed === "object" ? message.rejectedClosed.low || message.rejectedClosed.high : message.rejectedClosed !== 0))
                        writer.uint32(/* id 22, wireType 0 =*/176).uint64(message.rejectedClosed);
                    if (message.rejectedOther != null && $Object.hasOwnProperty.call(message, "rejectedOther") && (typeof message.rejectedOther === "object" ? message.rejectedOther.low || message.rejectedOther.high : message.rejectedOther !== 0))
                        writer.uint32(/* id 23, wireType 0 =*/184).uint64(message.rejectedOther);
                    if (message.retries != null && $Object.hasOwnProperty.call(message, "retries") && (typeof message.retries === "object" ? message.retries.low || message.retries.high : message.retries !== 0))
                        writer.uint32(/* id 24, wireType 0 =*/192).uint64(message.retries);
                    if (message.registrationHits != null && $Object.hasOwnProperty.call(message, "registrationHits") && (typeof message.registrationHits === "object" ? message.registrationHits.low || message.registrationHits.high : message.registrationHits !== 0))
                        writer.uint32(/* id 25, wireType 0 =*/200).uint64(message.registrationHits);
                    if (message.registrationMisses != null && $Object.hasOwnProperty.call(message, "registrationMisses") && (typeof message.registrationMisses === "object" ? message.registrationMisses.low || message.registrationMisses.high : message.registrationMisses !== 0))
                        writer.uint32(/* id 26, wireType 0 =*/208).uint64(message.registrationMisses);
                    if (message.registrationFailures != null && $Object.hasOwnProperty.call(message, "registrationFailures") && (typeof message.registrationFailures === "object" ? message.registrationFailures.low || message.registrationFailures.high : message.registrationFailures !== 0))
                        writer.uint32(/* id 27, wireType 0 =*/216).uint64(message.registrationFailures);
                    if (message.syncFailures != null && $Object.hasOwnProperty.call(message, "syncFailures") && (typeof message.syncFailures === "object" ? message.syncFailures.low || message.syncFailures.high : message.syncFailures !== 0))
                        writer.uint32(/* id 28, wireType 0 =*/224).uint64(message.syncFailures);
                    if (message.submitFailures != null && $Object.hasOwnProperty.call(message, "submitFailures") && (typeof message.submitFailures === "object" ? message.submitFailures.low || message.submitFailures.high : message.submitFailures !== 0))
                        writer.uint32(/* id 29, wireType 0 =*/232).uint64(message.submitFailures);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Ring message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Ring
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Ring & doramcap.storage.v1.Ring.$Shape} Ring
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Ring.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Ring();
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
                                    message.writer = value;
                                else
                                    delete message.writer;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.file = value;
                                else
                                    delete message.file;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.pendingCapacity = value;
                                else
                                    delete message.pendingCapacity;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.outstanding = value;
                                else
                                    delete message.outstanding;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.submitted = value;
                                else
                                    delete message.submitted;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.completions = value;
                                else
                                    delete message.completions;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.accepted = value;
                                else
                                    delete message.accepted;
                                continue;
                            }
                        case 8: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.completed = value;
                                else
                                    delete message.completed;
                                continue;
                            }
                        case 9: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.synced = value;
                                else
                                    delete message.synced;
                                continue;
                            }
                        case 10: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.awaitingSubmission = value;
                                else
                                    delete message.awaitingSubmission;
                                continue;
                            }
                        case 11: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.awaitingCqe = value;
                                else
                                    delete message.awaitingCqe;
                                continue;
                            }
                        case 12: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.pendingBytes = value;
                                else
                                    delete message.pendingBytes;
                                continue;
                            }
                        case 13: {
                                if (wireType !== 0)
                                    break;
                                message.oldestUs = reader.uint64();
                                message._oldestUs = "oldestUs";
                                continue;
                            }
                        case 14: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.registeredRegions = value;
                                else
                                    delete message.registeredRegions;
                                continue;
                            }
                        case 15: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.registrationUpdates = value;
                                else
                                    delete message.registrationUpdates;
                                continue;
                            }
                        case 16: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.acceptedWrites = value;
                                else
                                    delete message.acceptedWrites;
                                continue;
                            }
                        case 17: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.written = value;
                                else
                                    delete message.written;
                                continue;
                            }
                        case 18: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.failed = value;
                                else
                                    delete message.failed;
                                continue;
                            }
                        case 19: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.cancelled = value;
                                else
                                    delete message.cancelled;
                                continue;
                            }
                        case 20: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.rejectedCapacity = value;
                                else
                                    delete message.rejectedCapacity;
                                continue;
                            }
                        case 21: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.rejectedInvalid = value;
                                else
                                    delete message.rejectedInvalid;
                                continue;
                            }
                        case 22: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.rejectedClosed = value;
                                else
                                    delete message.rejectedClosed;
                                continue;
                            }
                        case 23: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.rejectedOther = value;
                                else
                                    delete message.rejectedOther;
                                continue;
                            }
                        case 24: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.retries = value;
                                else
                                    delete message.retries;
                                continue;
                            }
                        case 25: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.registrationHits = value;
                                else
                                    delete message.registrationHits;
                                continue;
                            }
                        case 26: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.registrationMisses = value;
                                else
                                    delete message.registrationMisses;
                                continue;
                            }
                        case 27: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.registrationFailures = value;
                                else
                                    delete message.registrationFailures;
                                continue;
                            }
                        case 28: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.syncFailures = value;
                                else
                                    delete message.syncFailures;
                                continue;
                            }
                        case 29: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.submitFailures = value;
                                else
                                    delete message.submitFailures;
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
                 * Creates a Ring message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Ring
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Ring} Ring
                 */
                Ring.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Ring)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Ring: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Ring();
                    if (object.writer != null)
                        if (typeof object.writer === "object" ? object.writer.low || object.writer.high : $Number(object.writer) !== 0)
                            if ($util.Long)
                                message.writer = $util.Long.fromValue(object.writer, true);
                            else if (typeof object.writer === "string")
                                message.writer = $parseInt(object.writer, 10);
                            else if (typeof object.writer === "number")
                                message.writer = object.writer;
                            else if (typeof object.writer === "object")
                                message.writer = new $util.LongBits(object.writer.low >>> 0, object.writer.high >>> 0).toNumber(true);
                    if (object.file != null)
                        if (typeof object.file === "object" ? object.file.low || object.file.high : $Number(object.file) !== 0)
                            if ($util.Long)
                                message.file = $util.Long.fromValue(object.file, true);
                            else if (typeof object.file === "string")
                                message.file = $parseInt(object.file, 10);
                            else if (typeof object.file === "number")
                                message.file = object.file;
                            else if (typeof object.file === "object")
                                message.file = new $util.LongBits(object.file.low >>> 0, object.file.high >>> 0).toNumber(true);
                    if (object.pendingCapacity != null)
                        if (typeof object.pendingCapacity === "object" ? object.pendingCapacity.low || object.pendingCapacity.high : $Number(object.pendingCapacity) !== 0)
                            if ($util.Long)
                                message.pendingCapacity = $util.Long.fromValue(object.pendingCapacity, true);
                            else if (typeof object.pendingCapacity === "string")
                                message.pendingCapacity = $parseInt(object.pendingCapacity, 10);
                            else if (typeof object.pendingCapacity === "number")
                                message.pendingCapacity = object.pendingCapacity;
                            else if (typeof object.pendingCapacity === "object")
                                message.pendingCapacity = new $util.LongBits(object.pendingCapacity.low >>> 0, object.pendingCapacity.high >>> 0).toNumber(true);
                    if (object.outstanding != null)
                        if (typeof object.outstanding === "object" ? object.outstanding.low || object.outstanding.high : $Number(object.outstanding) !== 0)
                            if ($util.Long)
                                message.outstanding = $util.Long.fromValue(object.outstanding, true);
                            else if (typeof object.outstanding === "string")
                                message.outstanding = $parseInt(object.outstanding, 10);
                            else if (typeof object.outstanding === "number")
                                message.outstanding = object.outstanding;
                            else if (typeof object.outstanding === "object")
                                message.outstanding = new $util.LongBits(object.outstanding.low >>> 0, object.outstanding.high >>> 0).toNumber(true);
                    if (object.submitted != null)
                        if (typeof object.submitted === "object" ? object.submitted.low || object.submitted.high : $Number(object.submitted) !== 0)
                            if ($util.Long)
                                message.submitted = $util.Long.fromValue(object.submitted, true);
                            else if (typeof object.submitted === "string")
                                message.submitted = $parseInt(object.submitted, 10);
                            else if (typeof object.submitted === "number")
                                message.submitted = object.submitted;
                            else if (typeof object.submitted === "object")
                                message.submitted = new $util.LongBits(object.submitted.low >>> 0, object.submitted.high >>> 0).toNumber(true);
                    if (object.completions != null)
                        if (typeof object.completions === "object" ? object.completions.low || object.completions.high : $Number(object.completions) !== 0)
                            if ($util.Long)
                                message.completions = $util.Long.fromValue(object.completions, true);
                            else if (typeof object.completions === "string")
                                message.completions = $parseInt(object.completions, 10);
                            else if (typeof object.completions === "number")
                                message.completions = object.completions;
                            else if (typeof object.completions === "object")
                                message.completions = new $util.LongBits(object.completions.low >>> 0, object.completions.high >>> 0).toNumber(true);
                    if (object.accepted != null)
                        if (typeof object.accepted === "object" ? object.accepted.low || object.accepted.high : $Number(object.accepted) !== 0)
                            if ($util.Long)
                                message.accepted = $util.Long.fromValue(object.accepted, true);
                            else if (typeof object.accepted === "string")
                                message.accepted = $parseInt(object.accepted, 10);
                            else if (typeof object.accepted === "number")
                                message.accepted = object.accepted;
                            else if (typeof object.accepted === "object")
                                message.accepted = new $util.LongBits(object.accepted.low >>> 0, object.accepted.high >>> 0).toNumber(true);
                    if (object.completed != null)
                        if (typeof object.completed === "object" ? object.completed.low || object.completed.high : $Number(object.completed) !== 0)
                            if ($util.Long)
                                message.completed = $util.Long.fromValue(object.completed, true);
                            else if (typeof object.completed === "string")
                                message.completed = $parseInt(object.completed, 10);
                            else if (typeof object.completed === "number")
                                message.completed = object.completed;
                            else if (typeof object.completed === "object")
                                message.completed = new $util.LongBits(object.completed.low >>> 0, object.completed.high >>> 0).toNumber(true);
                    if (object.synced != null)
                        if (typeof object.synced === "object" ? object.synced.low || object.synced.high : $Number(object.synced) !== 0)
                            if ($util.Long)
                                message.synced = $util.Long.fromValue(object.synced, true);
                            else if (typeof object.synced === "string")
                                message.synced = $parseInt(object.synced, 10);
                            else if (typeof object.synced === "number")
                                message.synced = object.synced;
                            else if (typeof object.synced === "object")
                                message.synced = new $util.LongBits(object.synced.low >>> 0, object.synced.high >>> 0).toNumber(true);
                    if (object.awaitingSubmission != null)
                        if (typeof object.awaitingSubmission === "object" ? object.awaitingSubmission.low || object.awaitingSubmission.high : $Number(object.awaitingSubmission) !== 0)
                            if ($util.Long)
                                message.awaitingSubmission = $util.Long.fromValue(object.awaitingSubmission, true);
                            else if (typeof object.awaitingSubmission === "string")
                                message.awaitingSubmission = $parseInt(object.awaitingSubmission, 10);
                            else if (typeof object.awaitingSubmission === "number")
                                message.awaitingSubmission = object.awaitingSubmission;
                            else if (typeof object.awaitingSubmission === "object")
                                message.awaitingSubmission = new $util.LongBits(object.awaitingSubmission.low >>> 0, object.awaitingSubmission.high >>> 0).toNumber(true);
                    if (object.awaitingCqe != null)
                        if (typeof object.awaitingCqe === "object" ? object.awaitingCqe.low || object.awaitingCqe.high : $Number(object.awaitingCqe) !== 0)
                            if ($util.Long)
                                message.awaitingCqe = $util.Long.fromValue(object.awaitingCqe, true);
                            else if (typeof object.awaitingCqe === "string")
                                message.awaitingCqe = $parseInt(object.awaitingCqe, 10);
                            else if (typeof object.awaitingCqe === "number")
                                message.awaitingCqe = object.awaitingCqe;
                            else if (typeof object.awaitingCqe === "object")
                                message.awaitingCqe = new $util.LongBits(object.awaitingCqe.low >>> 0, object.awaitingCqe.high >>> 0).toNumber(true);
                    if (object.pendingBytes != null)
                        if (typeof object.pendingBytes === "object" ? object.pendingBytes.low || object.pendingBytes.high : $Number(object.pendingBytes) !== 0)
                            if ($util.Long)
                                message.pendingBytes = $util.Long.fromValue(object.pendingBytes, true);
                            else if (typeof object.pendingBytes === "string")
                                message.pendingBytes = $parseInt(object.pendingBytes, 10);
                            else if (typeof object.pendingBytes === "number")
                                message.pendingBytes = object.pendingBytes;
                            else if (typeof object.pendingBytes === "object")
                                message.pendingBytes = new $util.LongBits(object.pendingBytes.low >>> 0, object.pendingBytes.high >>> 0).toNumber(true);
                    if (object.oldestUs != null)
                        if ($util.Long)
                            message.oldestUs = $util.Long.fromValue(object.oldestUs, true);
                        else if (typeof object.oldestUs === "string")
                            message.oldestUs = $parseInt(object.oldestUs, 10);
                        else if (typeof object.oldestUs === "number")
                            message.oldestUs = object.oldestUs;
                        else if (typeof object.oldestUs === "object")
                            message.oldestUs = new $util.LongBits(object.oldestUs.low >>> 0, object.oldestUs.high >>> 0).toNumber(true);
                    if (object.registeredRegions != null)
                        if (typeof object.registeredRegions === "object" ? object.registeredRegions.low || object.registeredRegions.high : $Number(object.registeredRegions) !== 0)
                            if ($util.Long)
                                message.registeredRegions = $util.Long.fromValue(object.registeredRegions, true);
                            else if (typeof object.registeredRegions === "string")
                                message.registeredRegions = $parseInt(object.registeredRegions, 10);
                            else if (typeof object.registeredRegions === "number")
                                message.registeredRegions = object.registeredRegions;
                            else if (typeof object.registeredRegions === "object")
                                message.registeredRegions = new $util.LongBits(object.registeredRegions.low >>> 0, object.registeredRegions.high >>> 0).toNumber(true);
                    if (object.registrationUpdates != null)
                        if (typeof object.registrationUpdates === "object" ? object.registrationUpdates.low || object.registrationUpdates.high : $Number(object.registrationUpdates) !== 0)
                            if ($util.Long)
                                message.registrationUpdates = $util.Long.fromValue(object.registrationUpdates, true);
                            else if (typeof object.registrationUpdates === "string")
                                message.registrationUpdates = $parseInt(object.registrationUpdates, 10);
                            else if (typeof object.registrationUpdates === "number")
                                message.registrationUpdates = object.registrationUpdates;
                            else if (typeof object.registrationUpdates === "object")
                                message.registrationUpdates = new $util.LongBits(object.registrationUpdates.low >>> 0, object.registrationUpdates.high >>> 0).toNumber(true);
                    if (object.acceptedWrites != null)
                        if (typeof object.acceptedWrites === "object" ? object.acceptedWrites.low || object.acceptedWrites.high : $Number(object.acceptedWrites) !== 0)
                            if ($util.Long)
                                message.acceptedWrites = $util.Long.fromValue(object.acceptedWrites, true);
                            else if (typeof object.acceptedWrites === "string")
                                message.acceptedWrites = $parseInt(object.acceptedWrites, 10);
                            else if (typeof object.acceptedWrites === "number")
                                message.acceptedWrites = object.acceptedWrites;
                            else if (typeof object.acceptedWrites === "object")
                                message.acceptedWrites = new $util.LongBits(object.acceptedWrites.low >>> 0, object.acceptedWrites.high >>> 0).toNumber(true);
                    if (object.written != null)
                        if (typeof object.written === "object" ? object.written.low || object.written.high : $Number(object.written) !== 0)
                            if ($util.Long)
                                message.written = $util.Long.fromValue(object.written, true);
                            else if (typeof object.written === "string")
                                message.written = $parseInt(object.written, 10);
                            else if (typeof object.written === "number")
                                message.written = object.written;
                            else if (typeof object.written === "object")
                                message.written = new $util.LongBits(object.written.low >>> 0, object.written.high >>> 0).toNumber(true);
                    if (object.failed != null)
                        if (typeof object.failed === "object" ? object.failed.low || object.failed.high : $Number(object.failed) !== 0)
                            if ($util.Long)
                                message.failed = $util.Long.fromValue(object.failed, true);
                            else if (typeof object.failed === "string")
                                message.failed = $parseInt(object.failed, 10);
                            else if (typeof object.failed === "number")
                                message.failed = object.failed;
                            else if (typeof object.failed === "object")
                                message.failed = new $util.LongBits(object.failed.low >>> 0, object.failed.high >>> 0).toNumber(true);
                    if (object.cancelled != null)
                        if (typeof object.cancelled === "object" ? object.cancelled.low || object.cancelled.high : $Number(object.cancelled) !== 0)
                            if ($util.Long)
                                message.cancelled = $util.Long.fromValue(object.cancelled, true);
                            else if (typeof object.cancelled === "string")
                                message.cancelled = $parseInt(object.cancelled, 10);
                            else if (typeof object.cancelled === "number")
                                message.cancelled = object.cancelled;
                            else if (typeof object.cancelled === "object")
                                message.cancelled = new $util.LongBits(object.cancelled.low >>> 0, object.cancelled.high >>> 0).toNumber(true);
                    if (object.rejectedCapacity != null)
                        if (typeof object.rejectedCapacity === "object" ? object.rejectedCapacity.low || object.rejectedCapacity.high : $Number(object.rejectedCapacity) !== 0)
                            if ($util.Long)
                                message.rejectedCapacity = $util.Long.fromValue(object.rejectedCapacity, true);
                            else if (typeof object.rejectedCapacity === "string")
                                message.rejectedCapacity = $parseInt(object.rejectedCapacity, 10);
                            else if (typeof object.rejectedCapacity === "number")
                                message.rejectedCapacity = object.rejectedCapacity;
                            else if (typeof object.rejectedCapacity === "object")
                                message.rejectedCapacity = new $util.LongBits(object.rejectedCapacity.low >>> 0, object.rejectedCapacity.high >>> 0).toNumber(true);
                    if (object.rejectedInvalid != null)
                        if (typeof object.rejectedInvalid === "object" ? object.rejectedInvalid.low || object.rejectedInvalid.high : $Number(object.rejectedInvalid) !== 0)
                            if ($util.Long)
                                message.rejectedInvalid = $util.Long.fromValue(object.rejectedInvalid, true);
                            else if (typeof object.rejectedInvalid === "string")
                                message.rejectedInvalid = $parseInt(object.rejectedInvalid, 10);
                            else if (typeof object.rejectedInvalid === "number")
                                message.rejectedInvalid = object.rejectedInvalid;
                            else if (typeof object.rejectedInvalid === "object")
                                message.rejectedInvalid = new $util.LongBits(object.rejectedInvalid.low >>> 0, object.rejectedInvalid.high >>> 0).toNumber(true);
                    if (object.rejectedClosed != null)
                        if (typeof object.rejectedClosed === "object" ? object.rejectedClosed.low || object.rejectedClosed.high : $Number(object.rejectedClosed) !== 0)
                            if ($util.Long)
                                message.rejectedClosed = $util.Long.fromValue(object.rejectedClosed, true);
                            else if (typeof object.rejectedClosed === "string")
                                message.rejectedClosed = $parseInt(object.rejectedClosed, 10);
                            else if (typeof object.rejectedClosed === "number")
                                message.rejectedClosed = object.rejectedClosed;
                            else if (typeof object.rejectedClosed === "object")
                                message.rejectedClosed = new $util.LongBits(object.rejectedClosed.low >>> 0, object.rejectedClosed.high >>> 0).toNumber(true);
                    if (object.rejectedOther != null)
                        if (typeof object.rejectedOther === "object" ? object.rejectedOther.low || object.rejectedOther.high : $Number(object.rejectedOther) !== 0)
                            if ($util.Long)
                                message.rejectedOther = $util.Long.fromValue(object.rejectedOther, true);
                            else if (typeof object.rejectedOther === "string")
                                message.rejectedOther = $parseInt(object.rejectedOther, 10);
                            else if (typeof object.rejectedOther === "number")
                                message.rejectedOther = object.rejectedOther;
                            else if (typeof object.rejectedOther === "object")
                                message.rejectedOther = new $util.LongBits(object.rejectedOther.low >>> 0, object.rejectedOther.high >>> 0).toNumber(true);
                    if (object.retries != null)
                        if (typeof object.retries === "object" ? object.retries.low || object.retries.high : $Number(object.retries) !== 0)
                            if ($util.Long)
                                message.retries = $util.Long.fromValue(object.retries, true);
                            else if (typeof object.retries === "string")
                                message.retries = $parseInt(object.retries, 10);
                            else if (typeof object.retries === "number")
                                message.retries = object.retries;
                            else if (typeof object.retries === "object")
                                message.retries = new $util.LongBits(object.retries.low >>> 0, object.retries.high >>> 0).toNumber(true);
                    if (object.registrationHits != null)
                        if (typeof object.registrationHits === "object" ? object.registrationHits.low || object.registrationHits.high : $Number(object.registrationHits) !== 0)
                            if ($util.Long)
                                message.registrationHits = $util.Long.fromValue(object.registrationHits, true);
                            else if (typeof object.registrationHits === "string")
                                message.registrationHits = $parseInt(object.registrationHits, 10);
                            else if (typeof object.registrationHits === "number")
                                message.registrationHits = object.registrationHits;
                            else if (typeof object.registrationHits === "object")
                                message.registrationHits = new $util.LongBits(object.registrationHits.low >>> 0, object.registrationHits.high >>> 0).toNumber(true);
                    if (object.registrationMisses != null)
                        if (typeof object.registrationMisses === "object" ? object.registrationMisses.low || object.registrationMisses.high : $Number(object.registrationMisses) !== 0)
                            if ($util.Long)
                                message.registrationMisses = $util.Long.fromValue(object.registrationMisses, true);
                            else if (typeof object.registrationMisses === "string")
                                message.registrationMisses = $parseInt(object.registrationMisses, 10);
                            else if (typeof object.registrationMisses === "number")
                                message.registrationMisses = object.registrationMisses;
                            else if (typeof object.registrationMisses === "object")
                                message.registrationMisses = new $util.LongBits(object.registrationMisses.low >>> 0, object.registrationMisses.high >>> 0).toNumber(true);
                    if (object.registrationFailures != null)
                        if (typeof object.registrationFailures === "object" ? object.registrationFailures.low || object.registrationFailures.high : $Number(object.registrationFailures) !== 0)
                            if ($util.Long)
                                message.registrationFailures = $util.Long.fromValue(object.registrationFailures, true);
                            else if (typeof object.registrationFailures === "string")
                                message.registrationFailures = $parseInt(object.registrationFailures, 10);
                            else if (typeof object.registrationFailures === "number")
                                message.registrationFailures = object.registrationFailures;
                            else if (typeof object.registrationFailures === "object")
                                message.registrationFailures = new $util.LongBits(object.registrationFailures.low >>> 0, object.registrationFailures.high >>> 0).toNumber(true);
                    if (object.syncFailures != null)
                        if (typeof object.syncFailures === "object" ? object.syncFailures.low || object.syncFailures.high : $Number(object.syncFailures) !== 0)
                            if ($util.Long)
                                message.syncFailures = $util.Long.fromValue(object.syncFailures, true);
                            else if (typeof object.syncFailures === "string")
                                message.syncFailures = $parseInt(object.syncFailures, 10);
                            else if (typeof object.syncFailures === "number")
                                message.syncFailures = object.syncFailures;
                            else if (typeof object.syncFailures === "object")
                                message.syncFailures = new $util.LongBits(object.syncFailures.low >>> 0, object.syncFailures.high >>> 0).toNumber(true);
                    if (object.submitFailures != null)
                        if (typeof object.submitFailures === "object" ? object.submitFailures.low || object.submitFailures.high : $Number(object.submitFailures) !== 0)
                            if ($util.Long)
                                message.submitFailures = $util.Long.fromValue(object.submitFailures, true);
                            else if (typeof object.submitFailures === "string")
                                message.submitFailures = $parseInt(object.submitFailures, 10);
                            else if (typeof object.submitFailures === "number")
                                message.submitFailures = object.submitFailures;
                            else if (typeof object.submitFailures === "object")
                                message.submitFailures = new $util.LongBits(object.submitFailures.low >>> 0, object.submitFailures.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a Ring message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Ring
                 * @static
                 * @param {doramcap.storage.v1.Ring} message Ring
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Ring.toObject = function (message, options, _depth) {
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
                            object.writer = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.writer = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.file = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.file = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.pendingCapacity = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.pendingCapacity = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.outstanding = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.outstanding = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.submitted = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.submitted = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.completions = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.completions = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.accepted = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.accepted = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.completed = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.completed = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.synced = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.synced = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.awaitingSubmission = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.awaitingSubmission = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.awaitingCqe = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.awaitingCqe = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.pendingBytes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.pendingBytes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.registeredRegions = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.registeredRegions = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.registrationUpdates = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.registrationUpdates = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.acceptedWrites = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.acceptedWrites = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.written = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.written = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.failed = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.failed = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.cancelled = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.cancelled = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.rejectedCapacity = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.rejectedCapacity = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.rejectedInvalid = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.rejectedInvalid = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.rejectedClosed = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.rejectedClosed = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.rejectedOther = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.rejectedOther = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.retries = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.retries = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.registrationHits = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.registrationHits = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.registrationMisses = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.registrationMisses = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.registrationFailures = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.registrationFailures = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.syncFailures = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.syncFailures = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.submitFailures = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.submitFailures = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.writer != null && $Object.hasOwnProperty.call(message, "writer"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.writer = typeof message.writer === "number" ? $BigInt(message.writer) : $util.Long.fromBits(message.writer.low >>> 0, message.writer.high >>> 0, true).toBigInt();
                        else if (typeof message.writer === "number")
                            object.writer = options.longs === $String ? $String(message.writer) : message.writer;
                        else
                            object.writer = options.longs === $String ? $util.Long.prototype.toString.call(message.writer) : options.longs === $Number ? new $util.LongBits(message.writer.low >>> 0, message.writer.high >>> 0).toNumber(true) : message.writer;
                    if (message.file != null && $Object.hasOwnProperty.call(message, "file"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.file = typeof message.file === "number" ? $BigInt(message.file) : $util.Long.fromBits(message.file.low >>> 0, message.file.high >>> 0, true).toBigInt();
                        else if (typeof message.file === "number")
                            object.file = options.longs === $String ? $String(message.file) : message.file;
                        else
                            object.file = options.longs === $String ? $util.Long.prototype.toString.call(message.file) : options.longs === $Number ? new $util.LongBits(message.file.low >>> 0, message.file.high >>> 0).toNumber(true) : message.file;
                    if (message.pendingCapacity != null && $Object.hasOwnProperty.call(message, "pendingCapacity"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.pendingCapacity = typeof message.pendingCapacity === "number" ? $BigInt(message.pendingCapacity) : $util.Long.fromBits(message.pendingCapacity.low >>> 0, message.pendingCapacity.high >>> 0, true).toBigInt();
                        else if (typeof message.pendingCapacity === "number")
                            object.pendingCapacity = options.longs === $String ? $String(message.pendingCapacity) : message.pendingCapacity;
                        else
                            object.pendingCapacity = options.longs === $String ? $util.Long.prototype.toString.call(message.pendingCapacity) : options.longs === $Number ? new $util.LongBits(message.pendingCapacity.low >>> 0, message.pendingCapacity.high >>> 0).toNumber(true) : message.pendingCapacity;
                    if (message.outstanding != null && $Object.hasOwnProperty.call(message, "outstanding"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.outstanding = typeof message.outstanding === "number" ? $BigInt(message.outstanding) : $util.Long.fromBits(message.outstanding.low >>> 0, message.outstanding.high >>> 0, true).toBigInt();
                        else if (typeof message.outstanding === "number")
                            object.outstanding = options.longs === $String ? $String(message.outstanding) : message.outstanding;
                        else
                            object.outstanding = options.longs === $String ? $util.Long.prototype.toString.call(message.outstanding) : options.longs === $Number ? new $util.LongBits(message.outstanding.low >>> 0, message.outstanding.high >>> 0).toNumber(true) : message.outstanding;
                    if (message.submitted != null && $Object.hasOwnProperty.call(message, "submitted"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.submitted = typeof message.submitted === "number" ? $BigInt(message.submitted) : $util.Long.fromBits(message.submitted.low >>> 0, message.submitted.high >>> 0, true).toBigInt();
                        else if (typeof message.submitted === "number")
                            object.submitted = options.longs === $String ? $String(message.submitted) : message.submitted;
                        else
                            object.submitted = options.longs === $String ? $util.Long.prototype.toString.call(message.submitted) : options.longs === $Number ? new $util.LongBits(message.submitted.low >>> 0, message.submitted.high >>> 0).toNumber(true) : message.submitted;
                    if (message.completions != null && $Object.hasOwnProperty.call(message, "completions"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.completions = typeof message.completions === "number" ? $BigInt(message.completions) : $util.Long.fromBits(message.completions.low >>> 0, message.completions.high >>> 0, true).toBigInt();
                        else if (typeof message.completions === "number")
                            object.completions = options.longs === $String ? $String(message.completions) : message.completions;
                        else
                            object.completions = options.longs === $String ? $util.Long.prototype.toString.call(message.completions) : options.longs === $Number ? new $util.LongBits(message.completions.low >>> 0, message.completions.high >>> 0).toNumber(true) : message.completions;
                    if (message.accepted != null && $Object.hasOwnProperty.call(message, "accepted"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.accepted = typeof message.accepted === "number" ? $BigInt(message.accepted) : $util.Long.fromBits(message.accepted.low >>> 0, message.accepted.high >>> 0, true).toBigInt();
                        else if (typeof message.accepted === "number")
                            object.accepted = options.longs === $String ? $String(message.accepted) : message.accepted;
                        else
                            object.accepted = options.longs === $String ? $util.Long.prototype.toString.call(message.accepted) : options.longs === $Number ? new $util.LongBits(message.accepted.low >>> 0, message.accepted.high >>> 0).toNumber(true) : message.accepted;
                    if (message.completed != null && $Object.hasOwnProperty.call(message, "completed"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.completed = typeof message.completed === "number" ? $BigInt(message.completed) : $util.Long.fromBits(message.completed.low >>> 0, message.completed.high >>> 0, true).toBigInt();
                        else if (typeof message.completed === "number")
                            object.completed = options.longs === $String ? $String(message.completed) : message.completed;
                        else
                            object.completed = options.longs === $String ? $util.Long.prototype.toString.call(message.completed) : options.longs === $Number ? new $util.LongBits(message.completed.low >>> 0, message.completed.high >>> 0).toNumber(true) : message.completed;
                    if (message.synced != null && $Object.hasOwnProperty.call(message, "synced"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.synced = typeof message.synced === "number" ? $BigInt(message.synced) : $util.Long.fromBits(message.synced.low >>> 0, message.synced.high >>> 0, true).toBigInt();
                        else if (typeof message.synced === "number")
                            object.synced = options.longs === $String ? $String(message.synced) : message.synced;
                        else
                            object.synced = options.longs === $String ? $util.Long.prototype.toString.call(message.synced) : options.longs === $Number ? new $util.LongBits(message.synced.low >>> 0, message.synced.high >>> 0).toNumber(true) : message.synced;
                    if (message.awaitingSubmission != null && $Object.hasOwnProperty.call(message, "awaitingSubmission"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.awaitingSubmission = typeof message.awaitingSubmission === "number" ? $BigInt(message.awaitingSubmission) : $util.Long.fromBits(message.awaitingSubmission.low >>> 0, message.awaitingSubmission.high >>> 0, true).toBigInt();
                        else if (typeof message.awaitingSubmission === "number")
                            object.awaitingSubmission = options.longs === $String ? $String(message.awaitingSubmission) : message.awaitingSubmission;
                        else
                            object.awaitingSubmission = options.longs === $String ? $util.Long.prototype.toString.call(message.awaitingSubmission) : options.longs === $Number ? new $util.LongBits(message.awaitingSubmission.low >>> 0, message.awaitingSubmission.high >>> 0).toNumber(true) : message.awaitingSubmission;
                    if (message.awaitingCqe != null && $Object.hasOwnProperty.call(message, "awaitingCqe"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.awaitingCqe = typeof message.awaitingCqe === "number" ? $BigInt(message.awaitingCqe) : $util.Long.fromBits(message.awaitingCqe.low >>> 0, message.awaitingCqe.high >>> 0, true).toBigInt();
                        else if (typeof message.awaitingCqe === "number")
                            object.awaitingCqe = options.longs === $String ? $String(message.awaitingCqe) : message.awaitingCqe;
                        else
                            object.awaitingCqe = options.longs === $String ? $util.Long.prototype.toString.call(message.awaitingCqe) : options.longs === $Number ? new $util.LongBits(message.awaitingCqe.low >>> 0, message.awaitingCqe.high >>> 0).toNumber(true) : message.awaitingCqe;
                    if (message.pendingBytes != null && $Object.hasOwnProperty.call(message, "pendingBytes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.pendingBytes = typeof message.pendingBytes === "number" ? $BigInt(message.pendingBytes) : $util.Long.fromBits(message.pendingBytes.low >>> 0, message.pendingBytes.high >>> 0, true).toBigInt();
                        else if (typeof message.pendingBytes === "number")
                            object.pendingBytes = options.longs === $String ? $String(message.pendingBytes) : message.pendingBytes;
                        else
                            object.pendingBytes = options.longs === $String ? $util.Long.prototype.toString.call(message.pendingBytes) : options.longs === $Number ? new $util.LongBits(message.pendingBytes.low >>> 0, message.pendingBytes.high >>> 0).toNumber(true) : message.pendingBytes;
                    if (message.oldestUs != null && $Object.hasOwnProperty.call(message, "oldestUs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.oldestUs = typeof message.oldestUs === "number" ? $BigInt(message.oldestUs) : $util.Long.fromBits(message.oldestUs.low >>> 0, message.oldestUs.high >>> 0, true).toBigInt();
                        else if (typeof message.oldestUs === "number")
                            object.oldestUs = options.longs === $String ? $String(message.oldestUs) : message.oldestUs;
                        else
                            object.oldestUs = options.longs === $String ? $util.Long.prototype.toString.call(message.oldestUs) : options.longs === $Number ? new $util.LongBits(message.oldestUs.low >>> 0, message.oldestUs.high >>> 0).toNumber(true) : message.oldestUs;
                    if (message.registeredRegions != null && $Object.hasOwnProperty.call(message, "registeredRegions"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.registeredRegions = typeof message.registeredRegions === "number" ? $BigInt(message.registeredRegions) : $util.Long.fromBits(message.registeredRegions.low >>> 0, message.registeredRegions.high >>> 0, true).toBigInt();
                        else if (typeof message.registeredRegions === "number")
                            object.registeredRegions = options.longs === $String ? $String(message.registeredRegions) : message.registeredRegions;
                        else
                            object.registeredRegions = options.longs === $String ? $util.Long.prototype.toString.call(message.registeredRegions) : options.longs === $Number ? new $util.LongBits(message.registeredRegions.low >>> 0, message.registeredRegions.high >>> 0).toNumber(true) : message.registeredRegions;
                    if (message.registrationUpdates != null && $Object.hasOwnProperty.call(message, "registrationUpdates"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.registrationUpdates = typeof message.registrationUpdates === "number" ? $BigInt(message.registrationUpdates) : $util.Long.fromBits(message.registrationUpdates.low >>> 0, message.registrationUpdates.high >>> 0, true).toBigInt();
                        else if (typeof message.registrationUpdates === "number")
                            object.registrationUpdates = options.longs === $String ? $String(message.registrationUpdates) : message.registrationUpdates;
                        else
                            object.registrationUpdates = options.longs === $String ? $util.Long.prototype.toString.call(message.registrationUpdates) : options.longs === $Number ? new $util.LongBits(message.registrationUpdates.low >>> 0, message.registrationUpdates.high >>> 0).toNumber(true) : message.registrationUpdates;
                    if (message.acceptedWrites != null && $Object.hasOwnProperty.call(message, "acceptedWrites"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.acceptedWrites = typeof message.acceptedWrites === "number" ? $BigInt(message.acceptedWrites) : $util.Long.fromBits(message.acceptedWrites.low >>> 0, message.acceptedWrites.high >>> 0, true).toBigInt();
                        else if (typeof message.acceptedWrites === "number")
                            object.acceptedWrites = options.longs === $String ? $String(message.acceptedWrites) : message.acceptedWrites;
                        else
                            object.acceptedWrites = options.longs === $String ? $util.Long.prototype.toString.call(message.acceptedWrites) : options.longs === $Number ? new $util.LongBits(message.acceptedWrites.low >>> 0, message.acceptedWrites.high >>> 0).toNumber(true) : message.acceptedWrites;
                    if (message.written != null && $Object.hasOwnProperty.call(message, "written"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.written = typeof message.written === "number" ? $BigInt(message.written) : $util.Long.fromBits(message.written.low >>> 0, message.written.high >>> 0, true).toBigInt();
                        else if (typeof message.written === "number")
                            object.written = options.longs === $String ? $String(message.written) : message.written;
                        else
                            object.written = options.longs === $String ? $util.Long.prototype.toString.call(message.written) : options.longs === $Number ? new $util.LongBits(message.written.low >>> 0, message.written.high >>> 0).toNumber(true) : message.written;
                    if (message.failed != null && $Object.hasOwnProperty.call(message, "failed"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.failed = typeof message.failed === "number" ? $BigInt(message.failed) : $util.Long.fromBits(message.failed.low >>> 0, message.failed.high >>> 0, true).toBigInt();
                        else if (typeof message.failed === "number")
                            object.failed = options.longs === $String ? $String(message.failed) : message.failed;
                        else
                            object.failed = options.longs === $String ? $util.Long.prototype.toString.call(message.failed) : options.longs === $Number ? new $util.LongBits(message.failed.low >>> 0, message.failed.high >>> 0).toNumber(true) : message.failed;
                    if (message.cancelled != null && $Object.hasOwnProperty.call(message, "cancelled"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.cancelled = typeof message.cancelled === "number" ? $BigInt(message.cancelled) : $util.Long.fromBits(message.cancelled.low >>> 0, message.cancelled.high >>> 0, true).toBigInt();
                        else if (typeof message.cancelled === "number")
                            object.cancelled = options.longs === $String ? $String(message.cancelled) : message.cancelled;
                        else
                            object.cancelled = options.longs === $String ? $util.Long.prototype.toString.call(message.cancelled) : options.longs === $Number ? new $util.LongBits(message.cancelled.low >>> 0, message.cancelled.high >>> 0).toNumber(true) : message.cancelled;
                    if (message.rejectedCapacity != null && $Object.hasOwnProperty.call(message, "rejectedCapacity"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.rejectedCapacity = typeof message.rejectedCapacity === "number" ? $BigInt(message.rejectedCapacity) : $util.Long.fromBits(message.rejectedCapacity.low >>> 0, message.rejectedCapacity.high >>> 0, true).toBigInt();
                        else if (typeof message.rejectedCapacity === "number")
                            object.rejectedCapacity = options.longs === $String ? $String(message.rejectedCapacity) : message.rejectedCapacity;
                        else
                            object.rejectedCapacity = options.longs === $String ? $util.Long.prototype.toString.call(message.rejectedCapacity) : options.longs === $Number ? new $util.LongBits(message.rejectedCapacity.low >>> 0, message.rejectedCapacity.high >>> 0).toNumber(true) : message.rejectedCapacity;
                    if (message.rejectedInvalid != null && $Object.hasOwnProperty.call(message, "rejectedInvalid"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.rejectedInvalid = typeof message.rejectedInvalid === "number" ? $BigInt(message.rejectedInvalid) : $util.Long.fromBits(message.rejectedInvalid.low >>> 0, message.rejectedInvalid.high >>> 0, true).toBigInt();
                        else if (typeof message.rejectedInvalid === "number")
                            object.rejectedInvalid = options.longs === $String ? $String(message.rejectedInvalid) : message.rejectedInvalid;
                        else
                            object.rejectedInvalid = options.longs === $String ? $util.Long.prototype.toString.call(message.rejectedInvalid) : options.longs === $Number ? new $util.LongBits(message.rejectedInvalid.low >>> 0, message.rejectedInvalid.high >>> 0).toNumber(true) : message.rejectedInvalid;
                    if (message.rejectedClosed != null && $Object.hasOwnProperty.call(message, "rejectedClosed"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.rejectedClosed = typeof message.rejectedClosed === "number" ? $BigInt(message.rejectedClosed) : $util.Long.fromBits(message.rejectedClosed.low >>> 0, message.rejectedClosed.high >>> 0, true).toBigInt();
                        else if (typeof message.rejectedClosed === "number")
                            object.rejectedClosed = options.longs === $String ? $String(message.rejectedClosed) : message.rejectedClosed;
                        else
                            object.rejectedClosed = options.longs === $String ? $util.Long.prototype.toString.call(message.rejectedClosed) : options.longs === $Number ? new $util.LongBits(message.rejectedClosed.low >>> 0, message.rejectedClosed.high >>> 0).toNumber(true) : message.rejectedClosed;
                    if (message.rejectedOther != null && $Object.hasOwnProperty.call(message, "rejectedOther"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.rejectedOther = typeof message.rejectedOther === "number" ? $BigInt(message.rejectedOther) : $util.Long.fromBits(message.rejectedOther.low >>> 0, message.rejectedOther.high >>> 0, true).toBigInt();
                        else if (typeof message.rejectedOther === "number")
                            object.rejectedOther = options.longs === $String ? $String(message.rejectedOther) : message.rejectedOther;
                        else
                            object.rejectedOther = options.longs === $String ? $util.Long.prototype.toString.call(message.rejectedOther) : options.longs === $Number ? new $util.LongBits(message.rejectedOther.low >>> 0, message.rejectedOther.high >>> 0).toNumber(true) : message.rejectedOther;
                    if (message.retries != null && $Object.hasOwnProperty.call(message, "retries"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.retries = typeof message.retries === "number" ? $BigInt(message.retries) : $util.Long.fromBits(message.retries.low >>> 0, message.retries.high >>> 0, true).toBigInt();
                        else if (typeof message.retries === "number")
                            object.retries = options.longs === $String ? $String(message.retries) : message.retries;
                        else
                            object.retries = options.longs === $String ? $util.Long.prototype.toString.call(message.retries) : options.longs === $Number ? new $util.LongBits(message.retries.low >>> 0, message.retries.high >>> 0).toNumber(true) : message.retries;
                    if (message.registrationHits != null && $Object.hasOwnProperty.call(message, "registrationHits"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.registrationHits = typeof message.registrationHits === "number" ? $BigInt(message.registrationHits) : $util.Long.fromBits(message.registrationHits.low >>> 0, message.registrationHits.high >>> 0, true).toBigInt();
                        else if (typeof message.registrationHits === "number")
                            object.registrationHits = options.longs === $String ? $String(message.registrationHits) : message.registrationHits;
                        else
                            object.registrationHits = options.longs === $String ? $util.Long.prototype.toString.call(message.registrationHits) : options.longs === $Number ? new $util.LongBits(message.registrationHits.low >>> 0, message.registrationHits.high >>> 0).toNumber(true) : message.registrationHits;
                    if (message.registrationMisses != null && $Object.hasOwnProperty.call(message, "registrationMisses"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.registrationMisses = typeof message.registrationMisses === "number" ? $BigInt(message.registrationMisses) : $util.Long.fromBits(message.registrationMisses.low >>> 0, message.registrationMisses.high >>> 0, true).toBigInt();
                        else if (typeof message.registrationMisses === "number")
                            object.registrationMisses = options.longs === $String ? $String(message.registrationMisses) : message.registrationMisses;
                        else
                            object.registrationMisses = options.longs === $String ? $util.Long.prototype.toString.call(message.registrationMisses) : options.longs === $Number ? new $util.LongBits(message.registrationMisses.low >>> 0, message.registrationMisses.high >>> 0).toNumber(true) : message.registrationMisses;
                    if (message.registrationFailures != null && $Object.hasOwnProperty.call(message, "registrationFailures"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.registrationFailures = typeof message.registrationFailures === "number" ? $BigInt(message.registrationFailures) : $util.Long.fromBits(message.registrationFailures.low >>> 0, message.registrationFailures.high >>> 0, true).toBigInt();
                        else if (typeof message.registrationFailures === "number")
                            object.registrationFailures = options.longs === $String ? $String(message.registrationFailures) : message.registrationFailures;
                        else
                            object.registrationFailures = options.longs === $String ? $util.Long.prototype.toString.call(message.registrationFailures) : options.longs === $Number ? new $util.LongBits(message.registrationFailures.low >>> 0, message.registrationFailures.high >>> 0).toNumber(true) : message.registrationFailures;
                    if (message.syncFailures != null && $Object.hasOwnProperty.call(message, "syncFailures"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.syncFailures = typeof message.syncFailures === "number" ? $BigInt(message.syncFailures) : $util.Long.fromBits(message.syncFailures.low >>> 0, message.syncFailures.high >>> 0, true).toBigInt();
                        else if (typeof message.syncFailures === "number")
                            object.syncFailures = options.longs === $String ? $String(message.syncFailures) : message.syncFailures;
                        else
                            object.syncFailures = options.longs === $String ? $util.Long.prototype.toString.call(message.syncFailures) : options.longs === $Number ? new $util.LongBits(message.syncFailures.low >>> 0, message.syncFailures.high >>> 0).toNumber(true) : message.syncFailures;
                    if (message.submitFailures != null && $Object.hasOwnProperty.call(message, "submitFailures"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.submitFailures = typeof message.submitFailures === "number" ? $BigInt(message.submitFailures) : $util.Long.fromBits(message.submitFailures.low >>> 0, message.submitFailures.high >>> 0, true).toBigInt();
                        else if (typeof message.submitFailures === "number")
                            object.submitFailures = options.longs === $String ? $String(message.submitFailures) : message.submitFailures;
                        else
                            object.submitFailures = options.longs === $String ? $util.Long.prototype.toString.call(message.submitFailures) : options.longs === $Number ? new $util.LongBits(message.submitFailures.low >>> 0, message.submitFailures.high >>> 0).toNumber(true) : message.submitFailures;
                    return object;
                };

                /**
                 * Converts this Ring to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Ring
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Ring.prototype.toJSON = function() {
                    return Ring.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Ring
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Ring
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Ring.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Ring";
                };

                return Ring;
            })();

            v1.Observation = (function() {

                /**
                 * Properties of an Observation.
                 * @typedef {Object} doramcap.storage.v1.Observation.$Properties
                 * @property {string|null} [mode] Observation mode
                 * @property {number|Long|null} [cursor] Observation cursor
                 * @property {number|Long|null} [missed] Observation missed
                 * @property {number|Long|null} [contentionDrops] Observation contentionDrops
                 * @property {number|Long|null} [overwritten] Observation overwritten
                 * @property {number|Long|null} [timelineEvicted] Observation timelineEvicted
                 * @property {number|Long|null} [capacity] Observation capacity
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of an Observation.
                 * @memberof doramcap.storage.v1
                 * @interface IObservation
                 * @augments doramcap.storage.v1.Observation.$Properties
                 * @deprecated Use doramcap.storage.v1.Observation.$Properties instead.
                 */

                /**
                 * Shape of an Observation.
                 * @typedef {doramcap.storage.v1.Observation.$Properties} doramcap.storage.v1.Observation.$Shape
                 */

                /**
                 * Constructs a new Observation.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents an Observation.
                 * @constructor
                 * @param {doramcap.storage.v1.Observation.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Observation = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Observation mode.
                 * @member {string} mode
                 * @memberof doramcap.storage.v1.Observation
                 * @instance
                 */
                Observation.prototype.mode = "";

                /**
                 * Observation cursor.
                 * @member {number|Long} cursor
                 * @memberof doramcap.storage.v1.Observation
                 * @instance
                 */
                Observation.prototype.cursor = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Observation missed.
                 * @member {number|Long} missed
                 * @memberof doramcap.storage.v1.Observation
                 * @instance
                 */
                Observation.prototype.missed = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Observation contentionDrops.
                 * @member {number|Long} contentionDrops
                 * @memberof doramcap.storage.v1.Observation
                 * @instance
                 */
                Observation.prototype.contentionDrops = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Observation overwritten.
                 * @member {number|Long} overwritten
                 * @memberof doramcap.storage.v1.Observation
                 * @instance
                 */
                Observation.prototype.overwritten = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Observation timelineEvicted.
                 * @member {number|Long} timelineEvicted
                 * @memberof doramcap.storage.v1.Observation
                 * @instance
                 */
                Observation.prototype.timelineEvicted = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Observation capacity.
                 * @member {number|Long} capacity
                 * @memberof doramcap.storage.v1.Observation
                 * @instance
                 */
                Observation.prototype.capacity = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified Observation message. Does not implicitly {@link doramcap.storage.v1.Observation.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Observation
                 * @static
                 * @param {doramcap.storage.v1.Observation.$Properties} message Observation message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Observation.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.mode != null && $Object.hasOwnProperty.call(message, "mode") && message.mode !== "")
                        writer.uint32(/* id 1, wireType 2 =*/10).string(message.mode);
                    if (message.cursor != null && $Object.hasOwnProperty.call(message, "cursor") && (typeof message.cursor === "object" ? message.cursor.low || message.cursor.high : message.cursor !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.cursor);
                    if (message.missed != null && $Object.hasOwnProperty.call(message, "missed") && (typeof message.missed === "object" ? message.missed.low || message.missed.high : message.missed !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.missed);
                    if (message.contentionDrops != null && $Object.hasOwnProperty.call(message, "contentionDrops") && (typeof message.contentionDrops === "object" ? message.contentionDrops.low || message.contentionDrops.high : message.contentionDrops !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.contentionDrops);
                    if (message.overwritten != null && $Object.hasOwnProperty.call(message, "overwritten") && (typeof message.overwritten === "object" ? message.overwritten.low || message.overwritten.high : message.overwritten !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.overwritten);
                    if (message.timelineEvicted != null && $Object.hasOwnProperty.call(message, "timelineEvicted") && (typeof message.timelineEvicted === "object" ? message.timelineEvicted.low || message.timelineEvicted.high : message.timelineEvicted !== 0))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.timelineEvicted);
                    if (message.capacity != null && $Object.hasOwnProperty.call(message, "capacity") && (typeof message.capacity === "object" ? message.capacity.low || message.capacity.high : message.capacity !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.capacity);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes an Observation message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Observation
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Observation & doramcap.storage.v1.Observation.$Shape} Observation
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Observation.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Observation();
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
                                    message.mode = value;
                                else
                                    delete message.mode;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.cursor = value;
                                else
                                    delete message.cursor;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.missed = value;
                                else
                                    delete message.missed;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.contentionDrops = value;
                                else
                                    delete message.contentionDrops;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.overwritten = value;
                                else
                                    delete message.overwritten;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.timelineEvicted = value;
                                else
                                    delete message.timelineEvicted;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.capacity = value;
                                else
                                    delete message.capacity;
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
                 * Creates an Observation message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Observation
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Observation} Observation
                 */
                Observation.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Observation)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Observation: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Observation();
                    if (object.mode != null)
                        if (typeof object.mode !== "string" || object.mode.length)
                            message.mode = $String(object.mode);
                    if (object.cursor != null)
                        if (typeof object.cursor === "object" ? object.cursor.low || object.cursor.high : $Number(object.cursor) !== 0)
                            if ($util.Long)
                                message.cursor = $util.Long.fromValue(object.cursor, true);
                            else if (typeof object.cursor === "string")
                                message.cursor = $parseInt(object.cursor, 10);
                            else if (typeof object.cursor === "number")
                                message.cursor = object.cursor;
                            else if (typeof object.cursor === "object")
                                message.cursor = new $util.LongBits(object.cursor.low >>> 0, object.cursor.high >>> 0).toNumber(true);
                    if (object.missed != null)
                        if (typeof object.missed === "object" ? object.missed.low || object.missed.high : $Number(object.missed) !== 0)
                            if ($util.Long)
                                message.missed = $util.Long.fromValue(object.missed, true);
                            else if (typeof object.missed === "string")
                                message.missed = $parseInt(object.missed, 10);
                            else if (typeof object.missed === "number")
                                message.missed = object.missed;
                            else if (typeof object.missed === "object")
                                message.missed = new $util.LongBits(object.missed.low >>> 0, object.missed.high >>> 0).toNumber(true);
                    if (object.contentionDrops != null)
                        if (typeof object.contentionDrops === "object" ? object.contentionDrops.low || object.contentionDrops.high : $Number(object.contentionDrops) !== 0)
                            if ($util.Long)
                                message.contentionDrops = $util.Long.fromValue(object.contentionDrops, true);
                            else if (typeof object.contentionDrops === "string")
                                message.contentionDrops = $parseInt(object.contentionDrops, 10);
                            else if (typeof object.contentionDrops === "number")
                                message.contentionDrops = object.contentionDrops;
                            else if (typeof object.contentionDrops === "object")
                                message.contentionDrops = new $util.LongBits(object.contentionDrops.low >>> 0, object.contentionDrops.high >>> 0).toNumber(true);
                    if (object.overwritten != null)
                        if (typeof object.overwritten === "object" ? object.overwritten.low || object.overwritten.high : $Number(object.overwritten) !== 0)
                            if ($util.Long)
                                message.overwritten = $util.Long.fromValue(object.overwritten, true);
                            else if (typeof object.overwritten === "string")
                                message.overwritten = $parseInt(object.overwritten, 10);
                            else if (typeof object.overwritten === "number")
                                message.overwritten = object.overwritten;
                            else if (typeof object.overwritten === "object")
                                message.overwritten = new $util.LongBits(object.overwritten.low >>> 0, object.overwritten.high >>> 0).toNumber(true);
                    if (object.timelineEvicted != null)
                        if (typeof object.timelineEvicted === "object" ? object.timelineEvicted.low || object.timelineEvicted.high : $Number(object.timelineEvicted) !== 0)
                            if ($util.Long)
                                message.timelineEvicted = $util.Long.fromValue(object.timelineEvicted, true);
                            else if (typeof object.timelineEvicted === "string")
                                message.timelineEvicted = $parseInt(object.timelineEvicted, 10);
                            else if (typeof object.timelineEvicted === "number")
                                message.timelineEvicted = object.timelineEvicted;
                            else if (typeof object.timelineEvicted === "object")
                                message.timelineEvicted = new $util.LongBits(object.timelineEvicted.low >>> 0, object.timelineEvicted.high >>> 0).toNumber(true);
                    if (object.capacity != null)
                        if (typeof object.capacity === "object" ? object.capacity.low || object.capacity.high : $Number(object.capacity) !== 0)
                            if ($util.Long)
                                message.capacity = $util.Long.fromValue(object.capacity, true);
                            else if (typeof object.capacity === "string")
                                message.capacity = $parseInt(object.capacity, 10);
                            else if (typeof object.capacity === "number")
                                message.capacity = object.capacity;
                            else if (typeof object.capacity === "object")
                                message.capacity = new $util.LongBits(object.capacity.low >>> 0, object.capacity.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from an Observation message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Observation
                 * @static
                 * @param {doramcap.storage.v1.Observation} message Observation
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Observation.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.mode = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.cursor = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.cursor = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.missed = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.missed = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.contentionDrops = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.contentionDrops = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.overwritten = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.overwritten = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.timelineEvicted = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.timelineEvicted = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.capacity = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.capacity = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.mode != null && $Object.hasOwnProperty.call(message, "mode"))
                        object.mode = message.mode;
                    if (message.cursor != null && $Object.hasOwnProperty.call(message, "cursor"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.cursor = typeof message.cursor === "number" ? $BigInt(message.cursor) : $util.Long.fromBits(message.cursor.low >>> 0, message.cursor.high >>> 0, true).toBigInt();
                        else if (typeof message.cursor === "number")
                            object.cursor = options.longs === $String ? $String(message.cursor) : message.cursor;
                        else
                            object.cursor = options.longs === $String ? $util.Long.prototype.toString.call(message.cursor) : options.longs === $Number ? new $util.LongBits(message.cursor.low >>> 0, message.cursor.high >>> 0).toNumber(true) : message.cursor;
                    if (message.missed != null && $Object.hasOwnProperty.call(message, "missed"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.missed = typeof message.missed === "number" ? $BigInt(message.missed) : $util.Long.fromBits(message.missed.low >>> 0, message.missed.high >>> 0, true).toBigInt();
                        else if (typeof message.missed === "number")
                            object.missed = options.longs === $String ? $String(message.missed) : message.missed;
                        else
                            object.missed = options.longs === $String ? $util.Long.prototype.toString.call(message.missed) : options.longs === $Number ? new $util.LongBits(message.missed.low >>> 0, message.missed.high >>> 0).toNumber(true) : message.missed;
                    if (message.contentionDrops != null && $Object.hasOwnProperty.call(message, "contentionDrops"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.contentionDrops = typeof message.contentionDrops === "number" ? $BigInt(message.contentionDrops) : $util.Long.fromBits(message.contentionDrops.low >>> 0, message.contentionDrops.high >>> 0, true).toBigInt();
                        else if (typeof message.contentionDrops === "number")
                            object.contentionDrops = options.longs === $String ? $String(message.contentionDrops) : message.contentionDrops;
                        else
                            object.contentionDrops = options.longs === $String ? $util.Long.prototype.toString.call(message.contentionDrops) : options.longs === $Number ? new $util.LongBits(message.contentionDrops.low >>> 0, message.contentionDrops.high >>> 0).toNumber(true) : message.contentionDrops;
                    if (message.overwritten != null && $Object.hasOwnProperty.call(message, "overwritten"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.overwritten = typeof message.overwritten === "number" ? $BigInt(message.overwritten) : $util.Long.fromBits(message.overwritten.low >>> 0, message.overwritten.high >>> 0, true).toBigInt();
                        else if (typeof message.overwritten === "number")
                            object.overwritten = options.longs === $String ? $String(message.overwritten) : message.overwritten;
                        else
                            object.overwritten = options.longs === $String ? $util.Long.prototype.toString.call(message.overwritten) : options.longs === $Number ? new $util.LongBits(message.overwritten.low >>> 0, message.overwritten.high >>> 0).toNumber(true) : message.overwritten;
                    if (message.timelineEvicted != null && $Object.hasOwnProperty.call(message, "timelineEvicted"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.timelineEvicted = typeof message.timelineEvicted === "number" ? $BigInt(message.timelineEvicted) : $util.Long.fromBits(message.timelineEvicted.low >>> 0, message.timelineEvicted.high >>> 0, true).toBigInt();
                        else if (typeof message.timelineEvicted === "number")
                            object.timelineEvicted = options.longs === $String ? $String(message.timelineEvicted) : message.timelineEvicted;
                        else
                            object.timelineEvicted = options.longs === $String ? $util.Long.prototype.toString.call(message.timelineEvicted) : options.longs === $Number ? new $util.LongBits(message.timelineEvicted.low >>> 0, message.timelineEvicted.high >>> 0).toNumber(true) : message.timelineEvicted;
                    if (message.capacity != null && $Object.hasOwnProperty.call(message, "capacity"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.capacity = typeof message.capacity === "number" ? $BigInt(message.capacity) : $util.Long.fromBits(message.capacity.low >>> 0, message.capacity.high >>> 0, true).toBigInt();
                        else if (typeof message.capacity === "number")
                            object.capacity = options.longs === $String ? $String(message.capacity) : message.capacity;
                        else
                            object.capacity = options.longs === $String ? $util.Long.prototype.toString.call(message.capacity) : options.longs === $Number ? new $util.LongBits(message.capacity.low >>> 0, message.capacity.high >>> 0).toNumber(true) : message.capacity;
                    return object;
                };

                /**
                 * Converts this Observation to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Observation
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Observation.prototype.toJSON = function() {
                    return Observation.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Observation
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Observation
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Observation.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Observation";
                };

                return Observation;
            })();

            v1.ActiveWrite = (function() {

                /**
                 * Properties of an ActiveWrite.
                 * @typedef {Object} doramcap.storage.v1.ActiveWrite.$Properties
                 * @property {number|Long|null} [write] ActiveWrite write
                 * @property {number|Long|null} [operation] ActiveWrite operation
                 * @property {number|Long|null} [region] ActiveWrite region
                 * @property {number|Long|null} [slabUse] ActiveWrite slabUse
                 * @property {string|null} [state] ActiveWrite state
                 * @property {number|Long|null} [offset] ActiveWrite offset
                 * @property {number|Long|null} [len] ActiveWrite len
                 * @property {number|Long|null} [completed] ActiveWrite completed
                 * @property {number|Long|null} [ageUs] ActiveWrite ageUs
                 * @property {number|Long|null} [attempts] ActiveWrite attempts
                 * @property {number|Long|null} [retries] ActiveWrite retries
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of an ActiveWrite.
                 * @memberof doramcap.storage.v1
                 * @interface IActiveWrite
                 * @augments doramcap.storage.v1.ActiveWrite.$Properties
                 * @deprecated Use doramcap.storage.v1.ActiveWrite.$Properties instead.
                 */

                /**
                 * Shape of an ActiveWrite.
                 * @typedef {doramcap.storage.v1.ActiveWrite.$Properties} doramcap.storage.v1.ActiveWrite.$Shape
                 */

                /**
                 * Constructs a new ActiveWrite.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents an ActiveWrite.
                 * @constructor
                 * @param {doramcap.storage.v1.ActiveWrite.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const ActiveWrite = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * ActiveWrite write.
                 * @member {number|Long} write
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.write = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * ActiveWrite operation.
                 * @member {number|Long} operation
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.operation = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * ActiveWrite region.
                 * @member {number|Long} region
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.region = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * ActiveWrite slabUse.
                 * @member {number|Long} slabUse
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.slabUse = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * ActiveWrite state.
                 * @member {string} state
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.state = "";

                /**
                 * ActiveWrite offset.
                 * @member {number|Long} offset
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.offset = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * ActiveWrite len.
                 * @member {number|Long} len
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.len = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * ActiveWrite completed.
                 * @member {number|Long} completed
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.completed = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * ActiveWrite ageUs.
                 * @member {number|Long|null|undefined} ageUs
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.ageUs = null;

                /**
                 * ActiveWrite attempts.
                 * @member {number|Long} attempts
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.attempts = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * ActiveWrite retries.
                 * @member {number|Long} retries
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 */
                ActiveWrite.prototype.retries = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(ActiveWrite.prototype, "_ageUs", {
                    get: $util.oneOfGetter($oneOfFields = ["ageUs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified ActiveWrite message. Does not implicitly {@link doramcap.storage.v1.ActiveWrite.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @static
                 * @param {doramcap.storage.v1.ActiveWrite.$Properties} message ActiveWrite message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                ActiveWrite.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.write != null && $Object.hasOwnProperty.call(message, "write") && (typeof message.write === "object" ? message.write.low || message.write.high : message.write !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.write);
                    if (message.operation != null && $Object.hasOwnProperty.call(message, "operation") && (typeof message.operation === "object" ? message.operation.low || message.operation.high : message.operation !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.operation);
                    if (message.region != null && $Object.hasOwnProperty.call(message, "region") && (typeof message.region === "object" ? message.region.low || message.region.high : message.region !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.region);
                    if (message.slabUse != null && $Object.hasOwnProperty.call(message, "slabUse") && (typeof message.slabUse === "object" ? message.slabUse.low || message.slabUse.high : message.slabUse !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.slabUse);
                    if (message.state != null && $Object.hasOwnProperty.call(message, "state") && message.state !== "")
                        writer.uint32(/* id 5, wireType 2 =*/42).string(message.state);
                    if (message.offset != null && $Object.hasOwnProperty.call(message, "offset") && (typeof message.offset === "object" ? message.offset.low || message.offset.high : message.offset !== 0))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.offset);
                    if (message.len != null && $Object.hasOwnProperty.call(message, "len") && (typeof message.len === "object" ? message.len.low || message.len.high : message.len !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.len);
                    if (message.completed != null && $Object.hasOwnProperty.call(message, "completed") && (typeof message.completed === "object" ? message.completed.low || message.completed.high : message.completed !== 0))
                        writer.uint32(/* id 8, wireType 0 =*/64).uint64(message.completed);
                    if (message.ageUs != null && $Object.hasOwnProperty.call(message, "ageUs"))
                        writer.uint32(/* id 9, wireType 0 =*/72).uint64(message.ageUs);
                    if (message.attempts != null && $Object.hasOwnProperty.call(message, "attempts") && (typeof message.attempts === "object" ? message.attempts.low || message.attempts.high : message.attempts !== 0))
                        writer.uint32(/* id 10, wireType 0 =*/80).uint64(message.attempts);
                    if (message.retries != null && $Object.hasOwnProperty.call(message, "retries") && (typeof message.retries === "object" ? message.retries.low || message.retries.high : message.retries !== 0))
                        writer.uint32(/* id 11, wireType 0 =*/88).uint64(message.retries);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes an ActiveWrite message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.ActiveWrite & doramcap.storage.v1.ActiveWrite.$Shape} ActiveWrite
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                ActiveWrite.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.ActiveWrite();
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
                                    message.write = value;
                                else
                                    delete message.write;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.operation = value;
                                else
                                    delete message.operation;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.region = value;
                                else
                                    delete message.region;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.slabUse = value;
                                else
                                    delete message.slabUse;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.state = value;
                                else
                                    delete message.state;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.offset = value;
                                else
                                    delete message.offset;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.len = value;
                                else
                                    delete message.len;
                                continue;
                            }
                        case 8: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.completed = value;
                                else
                                    delete message.completed;
                                continue;
                            }
                        case 9: {
                                if (wireType !== 0)
                                    break;
                                message.ageUs = reader.uint64();
                                message._ageUs = "ageUs";
                                continue;
                            }
                        case 10: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.attempts = value;
                                else
                                    delete message.attempts;
                                continue;
                            }
                        case 11: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.retries = value;
                                else
                                    delete message.retries;
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
                 * Creates an ActiveWrite message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.ActiveWrite} ActiveWrite
                 */
                ActiveWrite.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.ActiveWrite)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.ActiveWrite: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.ActiveWrite();
                    if (object.write != null)
                        if (typeof object.write === "object" ? object.write.low || object.write.high : $Number(object.write) !== 0)
                            if ($util.Long)
                                message.write = $util.Long.fromValue(object.write, true);
                            else if (typeof object.write === "string")
                                message.write = $parseInt(object.write, 10);
                            else if (typeof object.write === "number")
                                message.write = object.write;
                            else if (typeof object.write === "object")
                                message.write = new $util.LongBits(object.write.low >>> 0, object.write.high >>> 0).toNumber(true);
                    if (object.operation != null)
                        if (typeof object.operation === "object" ? object.operation.low || object.operation.high : $Number(object.operation) !== 0)
                            if ($util.Long)
                                message.operation = $util.Long.fromValue(object.operation, true);
                            else if (typeof object.operation === "string")
                                message.operation = $parseInt(object.operation, 10);
                            else if (typeof object.operation === "number")
                                message.operation = object.operation;
                            else if (typeof object.operation === "object")
                                message.operation = new $util.LongBits(object.operation.low >>> 0, object.operation.high >>> 0).toNumber(true);
                    if (object.region != null)
                        if (typeof object.region === "object" ? object.region.low || object.region.high : $Number(object.region) !== 0)
                            if ($util.Long)
                                message.region = $util.Long.fromValue(object.region, true);
                            else if (typeof object.region === "string")
                                message.region = $parseInt(object.region, 10);
                            else if (typeof object.region === "number")
                                message.region = object.region;
                            else if (typeof object.region === "object")
                                message.region = new $util.LongBits(object.region.low >>> 0, object.region.high >>> 0).toNumber(true);
                    if (object.slabUse != null)
                        if (typeof object.slabUse === "object" ? object.slabUse.low || object.slabUse.high : $Number(object.slabUse) !== 0)
                            if ($util.Long)
                                message.slabUse = $util.Long.fromValue(object.slabUse, true);
                            else if (typeof object.slabUse === "string")
                                message.slabUse = $parseInt(object.slabUse, 10);
                            else if (typeof object.slabUse === "number")
                                message.slabUse = object.slabUse;
                            else if (typeof object.slabUse === "object")
                                message.slabUse = new $util.LongBits(object.slabUse.low >>> 0, object.slabUse.high >>> 0).toNumber(true);
                    if (object.state != null)
                        if (typeof object.state !== "string" || object.state.length)
                            message.state = $String(object.state);
                    if (object.offset != null)
                        if (typeof object.offset === "object" ? object.offset.low || object.offset.high : $Number(object.offset) !== 0)
                            if ($util.Long)
                                message.offset = $util.Long.fromValue(object.offset, true);
                            else if (typeof object.offset === "string")
                                message.offset = $parseInt(object.offset, 10);
                            else if (typeof object.offset === "number")
                                message.offset = object.offset;
                            else if (typeof object.offset === "object")
                                message.offset = new $util.LongBits(object.offset.low >>> 0, object.offset.high >>> 0).toNumber(true);
                    if (object.len != null)
                        if (typeof object.len === "object" ? object.len.low || object.len.high : $Number(object.len) !== 0)
                            if ($util.Long)
                                message.len = $util.Long.fromValue(object.len, true);
                            else if (typeof object.len === "string")
                                message.len = $parseInt(object.len, 10);
                            else if (typeof object.len === "number")
                                message.len = object.len;
                            else if (typeof object.len === "object")
                                message.len = new $util.LongBits(object.len.low >>> 0, object.len.high >>> 0).toNumber(true);
                    if (object.completed != null)
                        if (typeof object.completed === "object" ? object.completed.low || object.completed.high : $Number(object.completed) !== 0)
                            if ($util.Long)
                                message.completed = $util.Long.fromValue(object.completed, true);
                            else if (typeof object.completed === "string")
                                message.completed = $parseInt(object.completed, 10);
                            else if (typeof object.completed === "number")
                                message.completed = object.completed;
                            else if (typeof object.completed === "object")
                                message.completed = new $util.LongBits(object.completed.low >>> 0, object.completed.high >>> 0).toNumber(true);
                    if (object.ageUs != null)
                        if ($util.Long)
                            message.ageUs = $util.Long.fromValue(object.ageUs, true);
                        else if (typeof object.ageUs === "string")
                            message.ageUs = $parseInt(object.ageUs, 10);
                        else if (typeof object.ageUs === "number")
                            message.ageUs = object.ageUs;
                        else if (typeof object.ageUs === "object")
                            message.ageUs = new $util.LongBits(object.ageUs.low >>> 0, object.ageUs.high >>> 0).toNumber(true);
                    if (object.attempts != null)
                        if (typeof object.attempts === "object" ? object.attempts.low || object.attempts.high : $Number(object.attempts) !== 0)
                            if ($util.Long)
                                message.attempts = $util.Long.fromValue(object.attempts, true);
                            else if (typeof object.attempts === "string")
                                message.attempts = $parseInt(object.attempts, 10);
                            else if (typeof object.attempts === "number")
                                message.attempts = object.attempts;
                            else if (typeof object.attempts === "object")
                                message.attempts = new $util.LongBits(object.attempts.low >>> 0, object.attempts.high >>> 0).toNumber(true);
                    if (object.retries != null)
                        if (typeof object.retries === "object" ? object.retries.low || object.retries.high : $Number(object.retries) !== 0)
                            if ($util.Long)
                                message.retries = $util.Long.fromValue(object.retries, true);
                            else if (typeof object.retries === "string")
                                message.retries = $parseInt(object.retries, 10);
                            else if (typeof object.retries === "number")
                                message.retries = object.retries;
                            else if (typeof object.retries === "object")
                                message.retries = new $util.LongBits(object.retries.low >>> 0, object.retries.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from an ActiveWrite message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @static
                 * @param {doramcap.storage.v1.ActiveWrite} message ActiveWrite
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                ActiveWrite.toObject = function (message, options, _depth) {
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
                            object.write = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.write = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.operation = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.operation = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.region = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.region = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.slabUse = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.slabUse = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.state = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.offset = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.offset = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.len = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.len = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.completed = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.completed = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.attempts = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.attempts = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.retries = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.retries = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.write != null && $Object.hasOwnProperty.call(message, "write"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.write = typeof message.write === "number" ? $BigInt(message.write) : $util.Long.fromBits(message.write.low >>> 0, message.write.high >>> 0, true).toBigInt();
                        else if (typeof message.write === "number")
                            object.write = options.longs === $String ? $String(message.write) : message.write;
                        else
                            object.write = options.longs === $String ? $util.Long.prototype.toString.call(message.write) : options.longs === $Number ? new $util.LongBits(message.write.low >>> 0, message.write.high >>> 0).toNumber(true) : message.write;
                    if (message.operation != null && $Object.hasOwnProperty.call(message, "operation"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.operation = typeof message.operation === "number" ? $BigInt(message.operation) : $util.Long.fromBits(message.operation.low >>> 0, message.operation.high >>> 0, true).toBigInt();
                        else if (typeof message.operation === "number")
                            object.operation = options.longs === $String ? $String(message.operation) : message.operation;
                        else
                            object.operation = options.longs === $String ? $util.Long.prototype.toString.call(message.operation) : options.longs === $Number ? new $util.LongBits(message.operation.low >>> 0, message.operation.high >>> 0).toNumber(true) : message.operation;
                    if (message.region != null && $Object.hasOwnProperty.call(message, "region"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.region = typeof message.region === "number" ? $BigInt(message.region) : $util.Long.fromBits(message.region.low >>> 0, message.region.high >>> 0, true).toBigInt();
                        else if (typeof message.region === "number")
                            object.region = options.longs === $String ? $String(message.region) : message.region;
                        else
                            object.region = options.longs === $String ? $util.Long.prototype.toString.call(message.region) : options.longs === $Number ? new $util.LongBits(message.region.low >>> 0, message.region.high >>> 0).toNumber(true) : message.region;
                    if (message.slabUse != null && $Object.hasOwnProperty.call(message, "slabUse"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.slabUse = typeof message.slabUse === "number" ? $BigInt(message.slabUse) : $util.Long.fromBits(message.slabUse.low >>> 0, message.slabUse.high >>> 0, true).toBigInt();
                        else if (typeof message.slabUse === "number")
                            object.slabUse = options.longs === $String ? $String(message.slabUse) : message.slabUse;
                        else
                            object.slabUse = options.longs === $String ? $util.Long.prototype.toString.call(message.slabUse) : options.longs === $Number ? new $util.LongBits(message.slabUse.low >>> 0, message.slabUse.high >>> 0).toNumber(true) : message.slabUse;
                    if (message.state != null && $Object.hasOwnProperty.call(message, "state"))
                        object.state = message.state;
                    if (message.offset != null && $Object.hasOwnProperty.call(message, "offset"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.offset = typeof message.offset === "number" ? $BigInt(message.offset) : $util.Long.fromBits(message.offset.low >>> 0, message.offset.high >>> 0, true).toBigInt();
                        else if (typeof message.offset === "number")
                            object.offset = options.longs === $String ? $String(message.offset) : message.offset;
                        else
                            object.offset = options.longs === $String ? $util.Long.prototype.toString.call(message.offset) : options.longs === $Number ? new $util.LongBits(message.offset.low >>> 0, message.offset.high >>> 0).toNumber(true) : message.offset;
                    if (message.len != null && $Object.hasOwnProperty.call(message, "len"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.len = typeof message.len === "number" ? $BigInt(message.len) : $util.Long.fromBits(message.len.low >>> 0, message.len.high >>> 0, true).toBigInt();
                        else if (typeof message.len === "number")
                            object.len = options.longs === $String ? $String(message.len) : message.len;
                        else
                            object.len = options.longs === $String ? $util.Long.prototype.toString.call(message.len) : options.longs === $Number ? new $util.LongBits(message.len.low >>> 0, message.len.high >>> 0).toNumber(true) : message.len;
                    if (message.completed != null && $Object.hasOwnProperty.call(message, "completed"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.completed = typeof message.completed === "number" ? $BigInt(message.completed) : $util.Long.fromBits(message.completed.low >>> 0, message.completed.high >>> 0, true).toBigInt();
                        else if (typeof message.completed === "number")
                            object.completed = options.longs === $String ? $String(message.completed) : message.completed;
                        else
                            object.completed = options.longs === $String ? $util.Long.prototype.toString.call(message.completed) : options.longs === $Number ? new $util.LongBits(message.completed.low >>> 0, message.completed.high >>> 0).toNumber(true) : message.completed;
                    if (message.ageUs != null && $Object.hasOwnProperty.call(message, "ageUs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.ageUs = typeof message.ageUs === "number" ? $BigInt(message.ageUs) : $util.Long.fromBits(message.ageUs.low >>> 0, message.ageUs.high >>> 0, true).toBigInt();
                        else if (typeof message.ageUs === "number")
                            object.ageUs = options.longs === $String ? $String(message.ageUs) : message.ageUs;
                        else
                            object.ageUs = options.longs === $String ? $util.Long.prototype.toString.call(message.ageUs) : options.longs === $Number ? new $util.LongBits(message.ageUs.low >>> 0, message.ageUs.high >>> 0).toNumber(true) : message.ageUs;
                    if (message.attempts != null && $Object.hasOwnProperty.call(message, "attempts"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.attempts = typeof message.attempts === "number" ? $BigInt(message.attempts) : $util.Long.fromBits(message.attempts.low >>> 0, message.attempts.high >>> 0, true).toBigInt();
                        else if (typeof message.attempts === "number")
                            object.attempts = options.longs === $String ? $String(message.attempts) : message.attempts;
                        else
                            object.attempts = options.longs === $String ? $util.Long.prototype.toString.call(message.attempts) : options.longs === $Number ? new $util.LongBits(message.attempts.low >>> 0, message.attempts.high >>> 0).toNumber(true) : message.attempts;
                    if (message.retries != null && $Object.hasOwnProperty.call(message, "retries"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.retries = typeof message.retries === "number" ? $BigInt(message.retries) : $util.Long.fromBits(message.retries.low >>> 0, message.retries.high >>> 0, true).toBigInt();
                        else if (typeof message.retries === "number")
                            object.retries = options.longs === $String ? $String(message.retries) : message.retries;
                        else
                            object.retries = options.longs === $String ? $util.Long.prototype.toString.call(message.retries) : options.longs === $Number ? new $util.LongBits(message.retries.low >>> 0, message.retries.high >>> 0).toNumber(true) : message.retries;
                    return object;
                };

                /**
                 * Converts this ActiveWrite to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                ActiveWrite.prototype.toJSON = function() {
                    return ActiveWrite.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for ActiveWrite
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.ActiveWrite
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                ActiveWrite.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.ActiveWrite";
                };

                return ActiveWrite;
            })();

            v1.Timing = (function() {

                /**
                 * Properties of a Timing.
                 * @typedef {Object} doramcap.storage.v1.Timing.$Properties
                 * @property {doramcap.storage.v1.Stats.$Properties|null} [sample] Timing sample
                 * @property {doramcap.storage.v1.Stats.$Properties|null} [arrival] Timing arrival
                 * @property {number|Long|null} [gaps] Timing gaps
                 * @property {number|null} [intervalMs] Timing intervalMs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Timing.
                 * @memberof doramcap.storage.v1
                 * @interface ITiming
                 * @augments doramcap.storage.v1.Timing.$Properties
                 * @deprecated Use doramcap.storage.v1.Timing.$Properties instead.
                 */

                /**
                 * Shape of a Timing.
                 * @typedef {doramcap.storage.v1.Timing.$Properties} doramcap.storage.v1.Timing.$Shape
                 */

                /**
                 * Constructs a new Timing.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a Timing.
                 * @constructor
                 * @param {doramcap.storage.v1.Timing.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Timing = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Timing sample.
                 * @member {doramcap.storage.v1.Stats.$Properties|null|undefined} sample
                 * @memberof doramcap.storage.v1.Timing
                 * @instance
                 */
                Timing.prototype.sample = null;

                /**
                 * Timing arrival.
                 * @member {doramcap.storage.v1.Stats.$Properties|null|undefined} arrival
                 * @memberof doramcap.storage.v1.Timing
                 * @instance
                 */
                Timing.prototype.arrival = null;

                /**
                 * Timing gaps.
                 * @member {number|Long} gaps
                 * @memberof doramcap.storage.v1.Timing
                 * @instance
                 */
                Timing.prototype.gaps = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Timing intervalMs.
                 * @member {number|null|undefined} intervalMs
                 * @memberof doramcap.storage.v1.Timing
                 * @instance
                 */
                Timing.prototype.intervalMs = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Timing.prototype, "_intervalMs", {
                    get: $util.oneOfGetter($oneOfFields = ["intervalMs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Timing message. Does not implicitly {@link doramcap.storage.v1.Timing.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Timing
                 * @static
                 * @param {doramcap.storage.v1.Timing.$Properties} message Timing message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Timing.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.sample != null && $Object.hasOwnProperty.call(message, "sample"))
                        $root.doramcap.storage.v1.Stats.encode(message.sample, writer.uint32(/* id 1, wireType 2 =*/10).fork(), _depth + 1).ldelim();
                    if (message.arrival != null && $Object.hasOwnProperty.call(message, "arrival"))
                        $root.doramcap.storage.v1.Stats.encode(message.arrival, writer.uint32(/* id 2, wireType 2 =*/18).fork(), _depth + 1).ldelim();
                    if (message.gaps != null && $Object.hasOwnProperty.call(message, "gaps") && (typeof message.gaps === "object" ? message.gaps.low || message.gaps.high : message.gaps !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.gaps);
                    if (message.intervalMs != null && $Object.hasOwnProperty.call(message, "intervalMs"))
                        writer.uint32(/* id 4, wireType 1 =*/33).double(message.intervalMs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Timing message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Timing
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Timing & doramcap.storage.v1.Timing.$Shape} Timing
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Timing.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Timing();
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
                                message.sample = $root.doramcap.storage.v1.Stats.decode(reader, reader.size(), $undefined, _depth + 1, message.sample);
                                continue;
                            }
                        case 2: {
                                if (wireType !== 2)
                                    break;
                                message.arrival = $root.doramcap.storage.v1.Stats.decode(reader, reader.size(), $undefined, _depth + 1, message.arrival);
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.gaps = value;
                                else
                                    delete message.gaps;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 1)
                                    break;
                                message.intervalMs = reader.double();
                                message._intervalMs = "intervalMs";
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
                 * Creates a Timing message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Timing
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Timing} Timing
                 */
                Timing.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Timing)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Timing: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Timing();
                    if (object.sample != null) {
                        if (!$util.isObject(object.sample))
                            throw $TypeError(".doramcap.storage.v1.Timing.sample: object expected");
                        message.sample = $root.doramcap.storage.v1.Stats.fromObject(object.sample, _depth + 1);
                    }
                    if (object.arrival != null) {
                        if (!$util.isObject(object.arrival))
                            throw $TypeError(".doramcap.storage.v1.Timing.arrival: object expected");
                        message.arrival = $root.doramcap.storage.v1.Stats.fromObject(object.arrival, _depth + 1);
                    }
                    if (object.gaps != null)
                        if (typeof object.gaps === "object" ? object.gaps.low || object.gaps.high : $Number(object.gaps) !== 0)
                            if ($util.Long)
                                message.gaps = $util.Long.fromValue(object.gaps, true);
                            else if (typeof object.gaps === "string")
                                message.gaps = $parseInt(object.gaps, 10);
                            else if (typeof object.gaps === "number")
                                message.gaps = object.gaps;
                            else if (typeof object.gaps === "object")
                                message.gaps = new $util.LongBits(object.gaps.low >>> 0, object.gaps.high >>> 0).toNumber(true);
                    if (object.intervalMs != null)
                        message.intervalMs = $Number(object.intervalMs);
                    return message;
                };

                /**
                 * Creates a plain object from a Timing message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Timing
                 * @static
                 * @param {doramcap.storage.v1.Timing} message Timing
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Timing.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.sample = null;
                        object.arrival = null;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.gaps = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.gaps = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.sample != null && $Object.hasOwnProperty.call(message, "sample"))
                        object.sample = $root.doramcap.storage.v1.Stats.toObject(message.sample, options, _depth + 1);
                    if (message.arrival != null && $Object.hasOwnProperty.call(message, "arrival"))
                        object.arrival = $root.doramcap.storage.v1.Stats.toObject(message.arrival, options, _depth + 1);
                    if (message.gaps != null && $Object.hasOwnProperty.call(message, "gaps"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.gaps = typeof message.gaps === "number" ? $BigInt(message.gaps) : $util.Long.fromBits(message.gaps.low >>> 0, message.gaps.high >>> 0, true).toBigInt();
                        else if (typeof message.gaps === "number")
                            object.gaps = options.longs === $String ? $String(message.gaps) : message.gaps;
                        else
                            object.gaps = options.longs === $String ? $util.Long.prototype.toString.call(message.gaps) : options.longs === $Number ? new $util.LongBits(message.gaps.low >>> 0, message.gaps.high >>> 0).toNumber(true) : message.gaps;
                    if (message.intervalMs != null && $Object.hasOwnProperty.call(message, "intervalMs"))
                        object.intervalMs = options.json && !$isFinite(message.intervalMs) ? $String(message.intervalMs) : message.intervalMs;
                    return object;
                };

                /**
                 * Converts this Timing to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Timing
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Timing.prototype.toJSON = function() {
                    return Timing.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Timing
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Timing
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Timing.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Timing";
                };

                return Timing;
            })();

            v1.Stats = (function() {

                /**
                 * Properties of a Stats.
                 * @typedef {Object} doramcap.storage.v1.Stats.$Properties
                 * @property {number|Long|null} [count] Stats count
                 * @property {number|null} [lastMs] Stats lastMs
                 * @property {number|null} [p99Ms] Stats p99Ms
                 * @property {number|null} [maxMs] Stats maxMs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Stats.
                 * @memberof doramcap.storage.v1
                 * @interface IStats
                 * @augments doramcap.storage.v1.Stats.$Properties
                 * @deprecated Use doramcap.storage.v1.Stats.$Properties instead.
                 */

                /**
                 * Shape of a Stats.
                 * @typedef {doramcap.storage.v1.Stats.$Properties} doramcap.storage.v1.Stats.$Shape
                 */

                /**
                 * Constructs a new Stats.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a Stats.
                 * @constructor
                 * @param {doramcap.storage.v1.Stats.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Stats = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Stats count.
                 * @member {number|Long} count
                 * @memberof doramcap.storage.v1.Stats
                 * @instance
                 */
                Stats.prototype.count = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Stats lastMs.
                 * @member {number|null|undefined} lastMs
                 * @memberof doramcap.storage.v1.Stats
                 * @instance
                 */
                Stats.prototype.lastMs = null;

                /**
                 * Stats p99Ms.
                 * @member {number|null|undefined} p99Ms
                 * @memberof doramcap.storage.v1.Stats
                 * @instance
                 */
                Stats.prototype.p99Ms = null;

                /**
                 * Stats maxMs.
                 * @member {number|null|undefined} maxMs
                 * @memberof doramcap.storage.v1.Stats
                 * @instance
                 */
                Stats.prototype.maxMs = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Stats.prototype, "_lastMs", {
                    get: $util.oneOfGetter($oneOfFields = ["lastMs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Stats.prototype, "_p99Ms", {
                    get: $util.oneOfGetter($oneOfFields = ["p99Ms"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Stats.prototype, "_maxMs", {
                    get: $util.oneOfGetter($oneOfFields = ["maxMs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Stats message. Does not implicitly {@link doramcap.storage.v1.Stats.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Stats
                 * @static
                 * @param {doramcap.storage.v1.Stats.$Properties} message Stats message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Stats.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.count != null && $Object.hasOwnProperty.call(message, "count") && (typeof message.count === "object" ? message.count.low || message.count.high : message.count !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.count);
                    if (message.lastMs != null && $Object.hasOwnProperty.call(message, "lastMs"))
                        writer.uint32(/* id 2, wireType 1 =*/17).double(message.lastMs);
                    if (message.p99Ms != null && $Object.hasOwnProperty.call(message, "p99Ms"))
                        writer.uint32(/* id 3, wireType 1 =*/25).double(message.p99Ms);
                    if (message.maxMs != null && $Object.hasOwnProperty.call(message, "maxMs"))
                        writer.uint32(/* id 4, wireType 1 =*/33).double(message.maxMs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Stats message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Stats
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Stats & doramcap.storage.v1.Stats.$Shape} Stats
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Stats.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Stats();
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
                                    message.count = value;
                                else
                                    delete message.count;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 1)
                                    break;
                                message.lastMs = reader.double();
                                message._lastMs = "lastMs";
                                continue;
                            }
                        case 3: {
                                if (wireType !== 1)
                                    break;
                                message.p99Ms = reader.double();
                                message._p99Ms = "p99Ms";
                                continue;
                            }
                        case 4: {
                                if (wireType !== 1)
                                    break;
                                message.maxMs = reader.double();
                                message._maxMs = "maxMs";
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
                 * Creates a Stats message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Stats
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Stats} Stats
                 */
                Stats.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Stats)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Stats: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Stats();
                    if (object.count != null)
                        if (typeof object.count === "object" ? object.count.low || object.count.high : $Number(object.count) !== 0)
                            if ($util.Long)
                                message.count = $util.Long.fromValue(object.count, true);
                            else if (typeof object.count === "string")
                                message.count = $parseInt(object.count, 10);
                            else if (typeof object.count === "number")
                                message.count = object.count;
                            else if (typeof object.count === "object")
                                message.count = new $util.LongBits(object.count.low >>> 0, object.count.high >>> 0).toNumber(true);
                    if (object.lastMs != null)
                        message.lastMs = $Number(object.lastMs);
                    if (object.p99Ms != null)
                        message.p99Ms = $Number(object.p99Ms);
                    if (object.maxMs != null)
                        message.maxMs = $Number(object.maxMs);
                    return message;
                };

                /**
                 * Creates a plain object from a Stats message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Stats
                 * @static
                 * @param {doramcap.storage.v1.Stats} message Stats
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Stats.toObject = function (message, options, _depth) {
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
                            object.count = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.count = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    if (message.count != null && $Object.hasOwnProperty.call(message, "count"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.count = typeof message.count === "number" ? $BigInt(message.count) : $util.Long.fromBits(message.count.low >>> 0, message.count.high >>> 0, true).toBigInt();
                        else if (typeof message.count === "number")
                            object.count = options.longs === $String ? $String(message.count) : message.count;
                        else
                            object.count = options.longs === $String ? $util.Long.prototype.toString.call(message.count) : options.longs === $Number ? new $util.LongBits(message.count.low >>> 0, message.count.high >>> 0).toNumber(true) : message.count;
                    if (message.lastMs != null && $Object.hasOwnProperty.call(message, "lastMs"))
                        object.lastMs = options.json && !$isFinite(message.lastMs) ? $String(message.lastMs) : message.lastMs;
                    if (message.p99Ms != null && $Object.hasOwnProperty.call(message, "p99Ms"))
                        object.p99Ms = options.json && !$isFinite(message.p99Ms) ? $String(message.p99Ms) : message.p99Ms;
                    if (message.maxMs != null && $Object.hasOwnProperty.call(message, "maxMs"))
                        object.maxMs = options.json && !$isFinite(message.maxMs) ? $String(message.maxMs) : message.maxMs;
                    return object;
                };

                /**
                 * Converts this Stats to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Stats
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Stats.prototype.toJSON = function() {
                    return Stats.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Stats
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Stats
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Stats.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Stats";
                };

                return Stats;
            })();

            v1.Receipt = (function() {

                /**
                 * Properties of a Receipt.
                 * @typedef {Object} doramcap.storage.v1.Receipt.$Properties
                 * @property {string|null} [kind] Receipt kind
                 * @property {number|Long|null} [count] Receipt count
                 * @property {number|Long|null} [bytes] Receipt bytes
                 * @property {number|Long|null} [first] Receipt first
                 * @property {number|Long|null} [last] Receipt last
                 * @property {number|null} [encodeMs] Receipt encodeMs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Receipt.
                 * @memberof doramcap.storage.v1
                 * @interface IReceipt
                 * @augments doramcap.storage.v1.Receipt.$Properties
                 * @deprecated Use doramcap.storage.v1.Receipt.$Properties instead.
                 */

                /**
                 * Shape of a Receipt.
                 * @typedef {doramcap.storage.v1.Receipt.$Properties} doramcap.storage.v1.Receipt.$Shape
                 */

                /**
                 * Constructs a new Receipt.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a Receipt.
                 * @constructor
                 * @param {doramcap.storage.v1.Receipt.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Receipt = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Receipt kind.
                 * @member {string} kind
                 * @memberof doramcap.storage.v1.Receipt
                 * @instance
                 */
                Receipt.prototype.kind = "";

                /**
                 * Receipt count.
                 * @member {number|Long} count
                 * @memberof doramcap.storage.v1.Receipt
                 * @instance
                 */
                Receipt.prototype.count = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Receipt bytes.
                 * @member {number|Long} bytes
                 * @memberof doramcap.storage.v1.Receipt
                 * @instance
                 */
                Receipt.prototype.bytes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Receipt first.
                 * @member {number|Long|null|undefined} first
                 * @memberof doramcap.storage.v1.Receipt
                 * @instance
                 */
                Receipt.prototype.first = null;

                /**
                 * Receipt last.
                 * @member {number|Long|null|undefined} last
                 * @memberof doramcap.storage.v1.Receipt
                 * @instance
                 */
                Receipt.prototype.last = null;

                /**
                 * Receipt encodeMs.
                 * @member {number} encodeMs
                 * @memberof doramcap.storage.v1.Receipt
                 * @instance
                 */
                Receipt.prototype.encodeMs = 0;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Receipt.prototype, "_first", {
                    get: $util.oneOfGetter($oneOfFields = ["first"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Receipt.prototype, "_last", {
                    get: $util.oneOfGetter($oneOfFields = ["last"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Receipt message. Does not implicitly {@link doramcap.storage.v1.Receipt.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Receipt
                 * @static
                 * @param {doramcap.storage.v1.Receipt.$Properties} message Receipt message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Receipt.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind") && message.kind !== "")
                        writer.uint32(/* id 1, wireType 2 =*/10).string(message.kind);
                    if (message.count != null && $Object.hasOwnProperty.call(message, "count") && (typeof message.count === "object" ? message.count.low || message.count.high : message.count !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.count);
                    if (message.bytes != null && $Object.hasOwnProperty.call(message, "bytes") && (typeof message.bytes === "object" ? message.bytes.low || message.bytes.high : message.bytes !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.bytes);
                    if (message.first != null && $Object.hasOwnProperty.call(message, "first"))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.first);
                    if (message.last != null && $Object.hasOwnProperty.call(message, "last"))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.last);
                    if (message.encodeMs != null && $Object.hasOwnProperty.call(message, "encodeMs") && !$Object.is(message.encodeMs, 0))
                        writer.uint32(/* id 6, wireType 1 =*/49).double(message.encodeMs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Receipt message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Receipt
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Receipt & doramcap.storage.v1.Receipt.$Shape} Receipt
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Receipt.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Receipt();
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
                                    message.kind = value;
                                else
                                    delete message.kind;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.count = value;
                                else
                                    delete message.count;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.bytes = value;
                                else
                                    delete message.bytes;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                message.first = reader.uint64();
                                message._first = "first";
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                message.last = reader.uint64();
                                message._last = "last";
                                continue;
                            }
                        case 6: {
                                if (wireType !== 1)
                                    break;
                                if (!$Object.is(value = reader.double(), 0))
                                    message.encodeMs = value;
                                else
                                    delete message.encodeMs;
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
                 * Creates a Receipt message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Receipt
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Receipt} Receipt
                 */
                Receipt.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Receipt)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Receipt: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Receipt();
                    if (object.kind != null)
                        if (typeof object.kind !== "string" || object.kind.length)
                            message.kind = $String(object.kind);
                    if (object.count != null)
                        if (typeof object.count === "object" ? object.count.low || object.count.high : $Number(object.count) !== 0)
                            if ($util.Long)
                                message.count = $util.Long.fromValue(object.count, true);
                            else if (typeof object.count === "string")
                                message.count = $parseInt(object.count, 10);
                            else if (typeof object.count === "number")
                                message.count = object.count;
                            else if (typeof object.count === "object")
                                message.count = new $util.LongBits(object.count.low >>> 0, object.count.high >>> 0).toNumber(true);
                    if (object.bytes != null)
                        if (typeof object.bytes === "object" ? object.bytes.low || object.bytes.high : $Number(object.bytes) !== 0)
                            if ($util.Long)
                                message.bytes = $util.Long.fromValue(object.bytes, true);
                            else if (typeof object.bytes === "string")
                                message.bytes = $parseInt(object.bytes, 10);
                            else if (typeof object.bytes === "number")
                                message.bytes = object.bytes;
                            else if (typeof object.bytes === "object")
                                message.bytes = new $util.LongBits(object.bytes.low >>> 0, object.bytes.high >>> 0).toNumber(true);
                    if (object.first != null)
                        if ($util.Long)
                            message.first = $util.Long.fromValue(object.first, true);
                        else if (typeof object.first === "string")
                            message.first = $parseInt(object.first, 10);
                        else if (typeof object.first === "number")
                            message.first = object.first;
                        else if (typeof object.first === "object")
                            message.first = new $util.LongBits(object.first.low >>> 0, object.first.high >>> 0).toNumber(true);
                    if (object.last != null)
                        if ($util.Long)
                            message.last = $util.Long.fromValue(object.last, true);
                        else if (typeof object.last === "string")
                            message.last = $parseInt(object.last, 10);
                        else if (typeof object.last === "number")
                            message.last = object.last;
                        else if (typeof object.last === "object")
                            message.last = new $util.LongBits(object.last.low >>> 0, object.last.high >>> 0).toNumber(true);
                    if (object.encodeMs != null)
                        if (!$Object.is($Number(object.encodeMs), 0))
                            message.encodeMs = $Number(object.encodeMs);
                    return message;
                };

                /**
                 * Creates a plain object from a Receipt message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Receipt
                 * @static
                 * @param {doramcap.storage.v1.Receipt} message Receipt
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Receipt.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.kind = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.count = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.count = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.bytes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.bytes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.encodeMs = 0;
                    }
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind"))
                        object.kind = message.kind;
                    if (message.count != null && $Object.hasOwnProperty.call(message, "count"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.count = typeof message.count === "number" ? $BigInt(message.count) : $util.Long.fromBits(message.count.low >>> 0, message.count.high >>> 0, true).toBigInt();
                        else if (typeof message.count === "number")
                            object.count = options.longs === $String ? $String(message.count) : message.count;
                        else
                            object.count = options.longs === $String ? $util.Long.prototype.toString.call(message.count) : options.longs === $Number ? new $util.LongBits(message.count.low >>> 0, message.count.high >>> 0).toNumber(true) : message.count;
                    if (message.bytes != null && $Object.hasOwnProperty.call(message, "bytes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.bytes = typeof message.bytes === "number" ? $BigInt(message.bytes) : $util.Long.fromBits(message.bytes.low >>> 0, message.bytes.high >>> 0, true).toBigInt();
                        else if (typeof message.bytes === "number")
                            object.bytes = options.longs === $String ? $String(message.bytes) : message.bytes;
                        else
                            object.bytes = options.longs === $String ? $util.Long.prototype.toString.call(message.bytes) : options.longs === $Number ? new $util.LongBits(message.bytes.low >>> 0, message.bytes.high >>> 0).toNumber(true) : message.bytes;
                    if (message.first != null && $Object.hasOwnProperty.call(message, "first"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.first = typeof message.first === "number" ? $BigInt(message.first) : $util.Long.fromBits(message.first.low >>> 0, message.first.high >>> 0, true).toBigInt();
                        else if (typeof message.first === "number")
                            object.first = options.longs === $String ? $String(message.first) : message.first;
                        else
                            object.first = options.longs === $String ? $util.Long.prototype.toString.call(message.first) : options.longs === $Number ? new $util.LongBits(message.first.low >>> 0, message.first.high >>> 0).toNumber(true) : message.first;
                    if (message.last != null && $Object.hasOwnProperty.call(message, "last"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.last = typeof message.last === "number" ? $BigInt(message.last) : $util.Long.fromBits(message.last.low >>> 0, message.last.high >>> 0, true).toBigInt();
                        else if (typeof message.last === "number")
                            object.last = options.longs === $String ? $String(message.last) : message.last;
                        else
                            object.last = options.longs === $String ? $util.Long.prototype.toString.call(message.last) : options.longs === $Number ? new $util.LongBits(message.last.low >>> 0, message.last.high >>> 0).toNumber(true) : message.last;
                    if (message.encodeMs != null && $Object.hasOwnProperty.call(message, "encodeMs"))
                        object.encodeMs = options.json && !$isFinite(message.encodeMs) ? $String(message.encodeMs) : message.encodeMs;
                    return object;
                };

                /**
                 * Converts this Receipt to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Receipt
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Receipt.prototype.toJSON = function() {
                    return Receipt.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Receipt
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Receipt
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Receipt.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Receipt";
                };

                return Receipt;
            })();

            v1.Input = (function() {

                /**
                 * Properties of an Input.
                 * @typedef {Object} doramcap.storage.v1.Input.$Properties
                 * @property {number|Long|null} [sequence] Input sequence
                 * @property {string|null} [kind] Input kind
                 * @property {number|Long|null} [bytes] Input bytes
                 * @property {number|Long|null} [timeMs] Input timeMs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of an Input.
                 * @memberof doramcap.storage.v1
                 * @interface IInput
                 * @augments doramcap.storage.v1.Input.$Properties
                 * @deprecated Use doramcap.storage.v1.Input.$Properties instead.
                 */

                /**
                 * Shape of an Input.
                 * @typedef {doramcap.storage.v1.Input.$Properties} doramcap.storage.v1.Input.$Shape
                 */

                /**
                 * Constructs a new Input.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents an Input.
                 * @constructor
                 * @param {doramcap.storage.v1.Input.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Input = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Input sequence.
                 * @member {number|Long} sequence
                 * @memberof doramcap.storage.v1.Input
                 * @instance
                 */
                Input.prototype.sequence = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Input kind.
                 * @member {string} kind
                 * @memberof doramcap.storage.v1.Input
                 * @instance
                 */
                Input.prototype.kind = "";

                /**
                 * Input bytes.
                 * @member {number|Long} bytes
                 * @memberof doramcap.storage.v1.Input
                 * @instance
                 */
                Input.prototype.bytes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Input timeMs.
                 * @member {number|Long} timeMs
                 * @memberof doramcap.storage.v1.Input
                 * @instance
                 */
                Input.prototype.timeMs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified Input message. Does not implicitly {@link doramcap.storage.v1.Input.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Input
                 * @static
                 * @param {doramcap.storage.v1.Input.$Properties} message Input message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Input.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence") && (typeof message.sequence === "object" ? message.sequence.low || message.sequence.high : message.sequence !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.sequence);
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind") && message.kind !== "")
                        writer.uint32(/* id 2, wireType 2 =*/18).string(message.kind);
                    if (message.bytes != null && $Object.hasOwnProperty.call(message, "bytes") && (typeof message.bytes === "object" ? message.bytes.low || message.bytes.high : message.bytes !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.bytes);
                    if (message.timeMs != null && $Object.hasOwnProperty.call(message, "timeMs") && (typeof message.timeMs === "object" ? message.timeMs.low || message.timeMs.high : message.timeMs !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.timeMs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes an Input message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Input
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Input & doramcap.storage.v1.Input.$Shape} Input
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Input.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Input();
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
                                    message.sequence = value;
                                else
                                    delete message.sequence;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.kind = value;
                                else
                                    delete message.kind;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.bytes = value;
                                else
                                    delete message.bytes;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.timeMs = value;
                                else
                                    delete message.timeMs;
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
                 * Creates an Input message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Input
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Input} Input
                 */
                Input.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Input)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Input: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Input();
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
                    if (object.kind != null)
                        if (typeof object.kind !== "string" || object.kind.length)
                            message.kind = $String(object.kind);
                    if (object.bytes != null)
                        if (typeof object.bytes === "object" ? object.bytes.low || object.bytes.high : $Number(object.bytes) !== 0)
                            if ($util.Long)
                                message.bytes = $util.Long.fromValue(object.bytes, true);
                            else if (typeof object.bytes === "string")
                                message.bytes = $parseInt(object.bytes, 10);
                            else if (typeof object.bytes === "number")
                                message.bytes = object.bytes;
                            else if (typeof object.bytes === "object")
                                message.bytes = new $util.LongBits(object.bytes.low >>> 0, object.bytes.high >>> 0).toNumber(true);
                    if (object.timeMs != null)
                        if (typeof object.timeMs === "object" ? object.timeMs.low || object.timeMs.high : $Number(object.timeMs) !== 0)
                            if ($util.Long)
                                message.timeMs = $util.Long.fromValue(object.timeMs, true);
                            else if (typeof object.timeMs === "string")
                                message.timeMs = $parseInt(object.timeMs, 10);
                            else if (typeof object.timeMs === "number")
                                message.timeMs = object.timeMs;
                            else if (typeof object.timeMs === "object")
                                message.timeMs = new $util.LongBits(object.timeMs.low >>> 0, object.timeMs.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from an Input message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Input
                 * @static
                 * @param {doramcap.storage.v1.Input} message Input
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Input.toObject = function (message, options, _depth) {
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
                            object.sequence = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.sequence = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.kind = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.bytes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.bytes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.timeMs = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.timeMs = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.sequence = typeof message.sequence === "number" ? $BigInt(message.sequence) : $util.Long.fromBits(message.sequence.low >>> 0, message.sequence.high >>> 0, true).toBigInt();
                        else if (typeof message.sequence === "number")
                            object.sequence = options.longs === $String ? $String(message.sequence) : message.sequence;
                        else
                            object.sequence = options.longs === $String ? $util.Long.prototype.toString.call(message.sequence) : options.longs === $Number ? new $util.LongBits(message.sequence.low >>> 0, message.sequence.high >>> 0).toNumber(true) : message.sequence;
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind"))
                        object.kind = message.kind;
                    if (message.bytes != null && $Object.hasOwnProperty.call(message, "bytes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.bytes = typeof message.bytes === "number" ? $BigInt(message.bytes) : $util.Long.fromBits(message.bytes.low >>> 0, message.bytes.high >>> 0, true).toBigInt();
                        else if (typeof message.bytes === "number")
                            object.bytes = options.longs === $String ? $String(message.bytes) : message.bytes;
                        else
                            object.bytes = options.longs === $String ? $util.Long.prototype.toString.call(message.bytes) : options.longs === $Number ? new $util.LongBits(message.bytes.low >>> 0, message.bytes.high >>> 0).toNumber(true) : message.bytes;
                    if (message.timeMs != null && $Object.hasOwnProperty.call(message, "timeMs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.timeMs = typeof message.timeMs === "number" ? $BigInt(message.timeMs) : $util.Long.fromBits(message.timeMs.low >>> 0, message.timeMs.high >>> 0, true).toBigInt();
                        else if (typeof message.timeMs === "number")
                            object.timeMs = options.longs === $String ? $String(message.timeMs) : message.timeMs;
                        else
                            object.timeMs = options.longs === $String ? $util.Long.prototype.toString.call(message.timeMs) : options.longs === $Number ? new $util.LongBits(message.timeMs.low >>> 0, message.timeMs.high >>> 0).toNumber(true) : message.timeMs;
                    return object;
                };

                /**
                 * Converts this Input to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Input
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Input.prototype.toJSON = function() {
                    return Input.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Input
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Input
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Input.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Input";
                };

                return Input;
            })();

            v1.Event = (function() {

                /**
                 * Properties of an Event.
                 * @typedef {Object} doramcap.storage.v1.Event.$Properties
                 * @property {number|Long|null} [id] Event id
                 * @property {string|null} [kind] Event kind
                 * @property {number|Long|null} [us] Event us
                 * @property {number|Long|null} [writer] Event writer
                 * @property {number|Long|null} [file] Event file
                 * @property {number|Long|null} [write] Event write
                 * @property {number|Long|null} [operation] Event operation
                 * @property {number|Long|null} [region] Event region
                 * @property {number|Long|null} [slabUse] Event slabUse
                 * @property {number|Long|null} [pool] Event pool
                 * @property {number|Long|null} [offset] Event offset
                 * @property {number|Long|null} [len] Event len
                 * @property {number|Long|null} [result] Event result
                 * @property {number|Long|null} [latencyUs] Event latencyUs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of an Event.
                 * @memberof doramcap.storage.v1
                 * @interface IEvent
                 * @augments doramcap.storage.v1.Event.$Properties
                 * @deprecated Use doramcap.storage.v1.Event.$Properties instead.
                 */

                /**
                 * Shape of an Event.
                 * @typedef {doramcap.storage.v1.Event.$Properties} doramcap.storage.v1.Event.$Shape
                 */

                /**
                 * Constructs a new Event.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents an Event.
                 * @constructor
                 * @param {doramcap.storage.v1.Event.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Event = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Event id.
                 * @member {number|Long} id
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.id = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event kind.
                 * @member {string} kind
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.kind = "";

                /**
                 * Event us.
                 * @member {number|Long} us
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.us = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event writer.
                 * @member {number|Long} writer
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.writer = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event file.
                 * @member {number|Long} file
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.file = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event write.
                 * @member {number|Long} write
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.write = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event operation.
                 * @member {number|Long} operation
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.operation = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event region.
                 * @member {number|Long} region
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.region = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event slabUse.
                 * @member {number|Long} slabUse
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.slabUse = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event pool.
                 * @member {number|Long} pool
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.pool = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event offset.
                 * @member {number|Long} offset
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.offset = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event len.
                 * @member {number|Long} len
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.len = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Event result.
                 * @member {number|Long|null|undefined} result
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.result = null;

                /**
                 * Event latencyUs.
                 * @member {number|Long|null|undefined} latencyUs
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 */
                Event.prototype.latencyUs = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Event.prototype, "_result", {
                    get: $util.oneOfGetter($oneOfFields = ["result"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Event.prototype, "_latencyUs", {
                    get: $util.oneOfGetter($oneOfFields = ["latencyUs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Event message. Does not implicitly {@link doramcap.storage.v1.Event.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Event
                 * @static
                 * @param {doramcap.storage.v1.Event.$Properties} message Event message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Event.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.id != null && $Object.hasOwnProperty.call(message, "id") && (typeof message.id === "object" ? message.id.low || message.id.high : message.id !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.id);
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind") && message.kind !== "")
                        writer.uint32(/* id 2, wireType 2 =*/18).string(message.kind);
                    if (message.us != null && $Object.hasOwnProperty.call(message, "us") && (typeof message.us === "object" ? message.us.low || message.us.high : message.us !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.us);
                    if (message.writer != null && $Object.hasOwnProperty.call(message, "writer") && (typeof message.writer === "object" ? message.writer.low || message.writer.high : message.writer !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.writer);
                    if (message.file != null && $Object.hasOwnProperty.call(message, "file") && (typeof message.file === "object" ? message.file.low || message.file.high : message.file !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.file);
                    if (message.write != null && $Object.hasOwnProperty.call(message, "write") && (typeof message.write === "object" ? message.write.low || message.write.high : message.write !== 0))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.write);
                    if (message.operation != null && $Object.hasOwnProperty.call(message, "operation") && (typeof message.operation === "object" ? message.operation.low || message.operation.high : message.operation !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.operation);
                    if (message.region != null && $Object.hasOwnProperty.call(message, "region") && (typeof message.region === "object" ? message.region.low || message.region.high : message.region !== 0))
                        writer.uint32(/* id 8, wireType 0 =*/64).uint64(message.region);
                    if (message.slabUse != null && $Object.hasOwnProperty.call(message, "slabUse") && (typeof message.slabUse === "object" ? message.slabUse.low || message.slabUse.high : message.slabUse !== 0))
                        writer.uint32(/* id 9, wireType 0 =*/72).uint64(message.slabUse);
                    if (message.pool != null && $Object.hasOwnProperty.call(message, "pool") && (typeof message.pool === "object" ? message.pool.low || message.pool.high : message.pool !== 0))
                        writer.uint32(/* id 10, wireType 0 =*/80).uint64(message.pool);
                    if (message.offset != null && $Object.hasOwnProperty.call(message, "offset") && (typeof message.offset === "object" ? message.offset.low || message.offset.high : message.offset !== 0))
                        writer.uint32(/* id 11, wireType 0 =*/88).uint64(message.offset);
                    if (message.len != null && $Object.hasOwnProperty.call(message, "len") && (typeof message.len === "object" ? message.len.low || message.len.high : message.len !== 0))
                        writer.uint32(/* id 12, wireType 0 =*/96).uint64(message.len);
                    if (message.result != null && $Object.hasOwnProperty.call(message, "result"))
                        writer.uint32(/* id 13, wireType 0 =*/104).sint64(message.result);
                    if (message.latencyUs != null && $Object.hasOwnProperty.call(message, "latencyUs"))
                        writer.uint32(/* id 14, wireType 0 =*/112).uint64(message.latencyUs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes an Event message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Event
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Event & doramcap.storage.v1.Event.$Shape} Event
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Event.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Event();
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
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.kind = value;
                                else
                                    delete message.kind;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.us = value;
                                else
                                    delete message.us;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.writer = value;
                                else
                                    delete message.writer;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.file = value;
                                else
                                    delete message.file;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.write = value;
                                else
                                    delete message.write;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.operation = value;
                                else
                                    delete message.operation;
                                continue;
                            }
                        case 8: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.region = value;
                                else
                                    delete message.region;
                                continue;
                            }
                        case 9: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.slabUse = value;
                                else
                                    delete message.slabUse;
                                continue;
                            }
                        case 10: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.pool = value;
                                else
                                    delete message.pool;
                                continue;
                            }
                        case 11: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.offset = value;
                                else
                                    delete message.offset;
                                continue;
                            }
                        case 12: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.len = value;
                                else
                                    delete message.len;
                                continue;
                            }
                        case 13: {
                                if (wireType !== 0)
                                    break;
                                message.result = reader.sint64();
                                message._result = "result";
                                continue;
                            }
                        case 14: {
                                if (wireType !== 0)
                                    break;
                                message.latencyUs = reader.uint64();
                                message._latencyUs = "latencyUs";
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
                 * Creates an Event message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Event
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Event} Event
                 */
                Event.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Event)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Event: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Event();
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
                    if (object.kind != null)
                        if (typeof object.kind !== "string" || object.kind.length)
                            message.kind = $String(object.kind);
                    if (object.us != null)
                        if (typeof object.us === "object" ? object.us.low || object.us.high : $Number(object.us) !== 0)
                            if ($util.Long)
                                message.us = $util.Long.fromValue(object.us, true);
                            else if (typeof object.us === "string")
                                message.us = $parseInt(object.us, 10);
                            else if (typeof object.us === "number")
                                message.us = object.us;
                            else if (typeof object.us === "object")
                                message.us = new $util.LongBits(object.us.low >>> 0, object.us.high >>> 0).toNumber(true);
                    if (object.writer != null)
                        if (typeof object.writer === "object" ? object.writer.low || object.writer.high : $Number(object.writer) !== 0)
                            if ($util.Long)
                                message.writer = $util.Long.fromValue(object.writer, true);
                            else if (typeof object.writer === "string")
                                message.writer = $parseInt(object.writer, 10);
                            else if (typeof object.writer === "number")
                                message.writer = object.writer;
                            else if (typeof object.writer === "object")
                                message.writer = new $util.LongBits(object.writer.low >>> 0, object.writer.high >>> 0).toNumber(true);
                    if (object.file != null)
                        if (typeof object.file === "object" ? object.file.low || object.file.high : $Number(object.file) !== 0)
                            if ($util.Long)
                                message.file = $util.Long.fromValue(object.file, true);
                            else if (typeof object.file === "string")
                                message.file = $parseInt(object.file, 10);
                            else if (typeof object.file === "number")
                                message.file = object.file;
                            else if (typeof object.file === "object")
                                message.file = new $util.LongBits(object.file.low >>> 0, object.file.high >>> 0).toNumber(true);
                    if (object.write != null)
                        if (typeof object.write === "object" ? object.write.low || object.write.high : $Number(object.write) !== 0)
                            if ($util.Long)
                                message.write = $util.Long.fromValue(object.write, true);
                            else if (typeof object.write === "string")
                                message.write = $parseInt(object.write, 10);
                            else if (typeof object.write === "number")
                                message.write = object.write;
                            else if (typeof object.write === "object")
                                message.write = new $util.LongBits(object.write.low >>> 0, object.write.high >>> 0).toNumber(true);
                    if (object.operation != null)
                        if (typeof object.operation === "object" ? object.operation.low || object.operation.high : $Number(object.operation) !== 0)
                            if ($util.Long)
                                message.operation = $util.Long.fromValue(object.operation, true);
                            else if (typeof object.operation === "string")
                                message.operation = $parseInt(object.operation, 10);
                            else if (typeof object.operation === "number")
                                message.operation = object.operation;
                            else if (typeof object.operation === "object")
                                message.operation = new $util.LongBits(object.operation.low >>> 0, object.operation.high >>> 0).toNumber(true);
                    if (object.region != null)
                        if (typeof object.region === "object" ? object.region.low || object.region.high : $Number(object.region) !== 0)
                            if ($util.Long)
                                message.region = $util.Long.fromValue(object.region, true);
                            else if (typeof object.region === "string")
                                message.region = $parseInt(object.region, 10);
                            else if (typeof object.region === "number")
                                message.region = object.region;
                            else if (typeof object.region === "object")
                                message.region = new $util.LongBits(object.region.low >>> 0, object.region.high >>> 0).toNumber(true);
                    if (object.slabUse != null)
                        if (typeof object.slabUse === "object" ? object.slabUse.low || object.slabUse.high : $Number(object.slabUse) !== 0)
                            if ($util.Long)
                                message.slabUse = $util.Long.fromValue(object.slabUse, true);
                            else if (typeof object.slabUse === "string")
                                message.slabUse = $parseInt(object.slabUse, 10);
                            else if (typeof object.slabUse === "number")
                                message.slabUse = object.slabUse;
                            else if (typeof object.slabUse === "object")
                                message.slabUse = new $util.LongBits(object.slabUse.low >>> 0, object.slabUse.high >>> 0).toNumber(true);
                    if (object.pool != null)
                        if (typeof object.pool === "object" ? object.pool.low || object.pool.high : $Number(object.pool) !== 0)
                            if ($util.Long)
                                message.pool = $util.Long.fromValue(object.pool, true);
                            else if (typeof object.pool === "string")
                                message.pool = $parseInt(object.pool, 10);
                            else if (typeof object.pool === "number")
                                message.pool = object.pool;
                            else if (typeof object.pool === "object")
                                message.pool = new $util.LongBits(object.pool.low >>> 0, object.pool.high >>> 0).toNumber(true);
                    if (object.offset != null)
                        if (typeof object.offset === "object" ? object.offset.low || object.offset.high : $Number(object.offset) !== 0)
                            if ($util.Long)
                                message.offset = $util.Long.fromValue(object.offset, true);
                            else if (typeof object.offset === "string")
                                message.offset = $parseInt(object.offset, 10);
                            else if (typeof object.offset === "number")
                                message.offset = object.offset;
                            else if (typeof object.offset === "object")
                                message.offset = new $util.LongBits(object.offset.low >>> 0, object.offset.high >>> 0).toNumber(true);
                    if (object.len != null)
                        if (typeof object.len === "object" ? object.len.low || object.len.high : $Number(object.len) !== 0)
                            if ($util.Long)
                                message.len = $util.Long.fromValue(object.len, true);
                            else if (typeof object.len === "string")
                                message.len = $parseInt(object.len, 10);
                            else if (typeof object.len === "number")
                                message.len = object.len;
                            else if (typeof object.len === "object")
                                message.len = new $util.LongBits(object.len.low >>> 0, object.len.high >>> 0).toNumber(true);
                    if (object.result != null)
                        if ($util.Long)
                            message.result = $util.Long.fromValue(object.result, false);
                        else if (typeof object.result === "string")
                            message.result = $parseInt(object.result, 10);
                        else if (typeof object.result === "number")
                            message.result = object.result;
                        else if (typeof object.result === "object")
                            message.result = new $util.LongBits(object.result.low >>> 0, object.result.high >>> 0).toNumber();
                    if (object.latencyUs != null)
                        if ($util.Long)
                            message.latencyUs = $util.Long.fromValue(object.latencyUs, true);
                        else if (typeof object.latencyUs === "string")
                            message.latencyUs = $parseInt(object.latencyUs, 10);
                        else if (typeof object.latencyUs === "number")
                            message.latencyUs = object.latencyUs;
                        else if (typeof object.latencyUs === "object")
                            message.latencyUs = new $util.LongBits(object.latencyUs.low >>> 0, object.latencyUs.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from an Event message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Event
                 * @static
                 * @param {doramcap.storage.v1.Event} message Event
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Event.toObject = function (message, options, _depth) {
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
                        object.kind = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.us = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.us = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.writer = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.writer = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.file = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.file = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.write = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.write = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.operation = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.operation = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.region = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.region = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.slabUse = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.slabUse = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.pool = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.pool = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.offset = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.offset = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.len = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.len = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.id != null && $Object.hasOwnProperty.call(message, "id"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.id = typeof message.id === "number" ? $BigInt(message.id) : $util.Long.fromBits(message.id.low >>> 0, message.id.high >>> 0, true).toBigInt();
                        else if (typeof message.id === "number")
                            object.id = options.longs === $String ? $String(message.id) : message.id;
                        else
                            object.id = options.longs === $String ? $util.Long.prototype.toString.call(message.id) : options.longs === $Number ? new $util.LongBits(message.id.low >>> 0, message.id.high >>> 0).toNumber(true) : message.id;
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind"))
                        object.kind = message.kind;
                    if (message.us != null && $Object.hasOwnProperty.call(message, "us"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.us = typeof message.us === "number" ? $BigInt(message.us) : $util.Long.fromBits(message.us.low >>> 0, message.us.high >>> 0, true).toBigInt();
                        else if (typeof message.us === "number")
                            object.us = options.longs === $String ? $String(message.us) : message.us;
                        else
                            object.us = options.longs === $String ? $util.Long.prototype.toString.call(message.us) : options.longs === $Number ? new $util.LongBits(message.us.low >>> 0, message.us.high >>> 0).toNumber(true) : message.us;
                    if (message.writer != null && $Object.hasOwnProperty.call(message, "writer"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.writer = typeof message.writer === "number" ? $BigInt(message.writer) : $util.Long.fromBits(message.writer.low >>> 0, message.writer.high >>> 0, true).toBigInt();
                        else if (typeof message.writer === "number")
                            object.writer = options.longs === $String ? $String(message.writer) : message.writer;
                        else
                            object.writer = options.longs === $String ? $util.Long.prototype.toString.call(message.writer) : options.longs === $Number ? new $util.LongBits(message.writer.low >>> 0, message.writer.high >>> 0).toNumber(true) : message.writer;
                    if (message.file != null && $Object.hasOwnProperty.call(message, "file"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.file = typeof message.file === "number" ? $BigInt(message.file) : $util.Long.fromBits(message.file.low >>> 0, message.file.high >>> 0, true).toBigInt();
                        else if (typeof message.file === "number")
                            object.file = options.longs === $String ? $String(message.file) : message.file;
                        else
                            object.file = options.longs === $String ? $util.Long.prototype.toString.call(message.file) : options.longs === $Number ? new $util.LongBits(message.file.low >>> 0, message.file.high >>> 0).toNumber(true) : message.file;
                    if (message.write != null && $Object.hasOwnProperty.call(message, "write"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.write = typeof message.write === "number" ? $BigInt(message.write) : $util.Long.fromBits(message.write.low >>> 0, message.write.high >>> 0, true).toBigInt();
                        else if (typeof message.write === "number")
                            object.write = options.longs === $String ? $String(message.write) : message.write;
                        else
                            object.write = options.longs === $String ? $util.Long.prototype.toString.call(message.write) : options.longs === $Number ? new $util.LongBits(message.write.low >>> 0, message.write.high >>> 0).toNumber(true) : message.write;
                    if (message.operation != null && $Object.hasOwnProperty.call(message, "operation"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.operation = typeof message.operation === "number" ? $BigInt(message.operation) : $util.Long.fromBits(message.operation.low >>> 0, message.operation.high >>> 0, true).toBigInt();
                        else if (typeof message.operation === "number")
                            object.operation = options.longs === $String ? $String(message.operation) : message.operation;
                        else
                            object.operation = options.longs === $String ? $util.Long.prototype.toString.call(message.operation) : options.longs === $Number ? new $util.LongBits(message.operation.low >>> 0, message.operation.high >>> 0).toNumber(true) : message.operation;
                    if (message.region != null && $Object.hasOwnProperty.call(message, "region"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.region = typeof message.region === "number" ? $BigInt(message.region) : $util.Long.fromBits(message.region.low >>> 0, message.region.high >>> 0, true).toBigInt();
                        else if (typeof message.region === "number")
                            object.region = options.longs === $String ? $String(message.region) : message.region;
                        else
                            object.region = options.longs === $String ? $util.Long.prototype.toString.call(message.region) : options.longs === $Number ? new $util.LongBits(message.region.low >>> 0, message.region.high >>> 0).toNumber(true) : message.region;
                    if (message.slabUse != null && $Object.hasOwnProperty.call(message, "slabUse"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.slabUse = typeof message.slabUse === "number" ? $BigInt(message.slabUse) : $util.Long.fromBits(message.slabUse.low >>> 0, message.slabUse.high >>> 0, true).toBigInt();
                        else if (typeof message.slabUse === "number")
                            object.slabUse = options.longs === $String ? $String(message.slabUse) : message.slabUse;
                        else
                            object.slabUse = options.longs === $String ? $util.Long.prototype.toString.call(message.slabUse) : options.longs === $Number ? new $util.LongBits(message.slabUse.low >>> 0, message.slabUse.high >>> 0).toNumber(true) : message.slabUse;
                    if (message.pool != null && $Object.hasOwnProperty.call(message, "pool"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.pool = typeof message.pool === "number" ? $BigInt(message.pool) : $util.Long.fromBits(message.pool.low >>> 0, message.pool.high >>> 0, true).toBigInt();
                        else if (typeof message.pool === "number")
                            object.pool = options.longs === $String ? $String(message.pool) : message.pool;
                        else
                            object.pool = options.longs === $String ? $util.Long.prototype.toString.call(message.pool) : options.longs === $Number ? new $util.LongBits(message.pool.low >>> 0, message.pool.high >>> 0).toNumber(true) : message.pool;
                    if (message.offset != null && $Object.hasOwnProperty.call(message, "offset"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.offset = typeof message.offset === "number" ? $BigInt(message.offset) : $util.Long.fromBits(message.offset.low >>> 0, message.offset.high >>> 0, true).toBigInt();
                        else if (typeof message.offset === "number")
                            object.offset = options.longs === $String ? $String(message.offset) : message.offset;
                        else
                            object.offset = options.longs === $String ? $util.Long.prototype.toString.call(message.offset) : options.longs === $Number ? new $util.LongBits(message.offset.low >>> 0, message.offset.high >>> 0).toNumber(true) : message.offset;
                    if (message.len != null && $Object.hasOwnProperty.call(message, "len"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.len = typeof message.len === "number" ? $BigInt(message.len) : $util.Long.fromBits(message.len.low >>> 0, message.len.high >>> 0, true).toBigInt();
                        else if (typeof message.len === "number")
                            object.len = options.longs === $String ? $String(message.len) : message.len;
                        else
                            object.len = options.longs === $String ? $util.Long.prototype.toString.call(message.len) : options.longs === $Number ? new $util.LongBits(message.len.low >>> 0, message.len.high >>> 0).toNumber(true) : message.len;
                    if (message.result != null && $Object.hasOwnProperty.call(message, "result"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.result = typeof message.result === "number" ? $BigInt(message.result) : $util.Long.fromBits(message.result.low >>> 0, message.result.high >>> 0, false).toBigInt();
                        else if (typeof message.result === "number")
                            object.result = options.longs === $String ? $String(message.result) : message.result;
                        else
                            object.result = options.longs === $String ? $util.Long.prototype.toString.call(message.result) : options.longs === $Number ? new $util.LongBits(message.result.low >>> 0, message.result.high >>> 0).toNumber() : message.result;
                    if (message.latencyUs != null && $Object.hasOwnProperty.call(message, "latencyUs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.latencyUs = typeof message.latencyUs === "number" ? $BigInt(message.latencyUs) : $util.Long.fromBits(message.latencyUs.low >>> 0, message.latencyUs.high >>> 0, true).toBigInt();
                        else if (typeof message.latencyUs === "number")
                            object.latencyUs = options.longs === $String ? $String(message.latencyUs) : message.latencyUs;
                        else
                            object.latencyUs = options.longs === $String ? $util.Long.prototype.toString.call(message.latencyUs) : options.longs === $Number ? new $util.LongBits(message.latencyUs.low >>> 0, message.latencyUs.high >>> 0).toNumber(true) : message.latencyUs;
                    return object;
                };

                /**
                 * Converts this Event to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Event
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Event.prototype.toJSON = function() {
                    return Event.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Event
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Event
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Event.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Event";
                };

                return Event;
            })();

            v1.File = (function() {

                /**
                 * Properties of a File.
                 * @typedef {Object} doramcap.storage.v1.File.$Properties
                 * @property {number|Long|null} [index] File index
                 * @property {string|null} [name] File name
                 * @property {boolean|null} [sealed] File sealed
                 * @property {number|Long|null} [size] File size
                 * @property {number|Long|null} [parsedThrough] File parsedThrough
                 * @property {Array.<doramcap.storage.v1.Record.$Properties>|null} [records] File records
                 * @property {number|Long|null} [tailOffset] File tailOffset
                 * @property {Uint8Array|null} [tail] File tail
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a File.
                 * @memberof doramcap.storage.v1
                 * @interface IFile
                 * @augments doramcap.storage.v1.File.$Properties
                 * @deprecated Use doramcap.storage.v1.File.$Properties instead.
                 */

                /**
                 * Shape of a File.
                 * @typedef {doramcap.storage.v1.File.$Properties} doramcap.storage.v1.File.$Shape
                 */

                /**
                 * Constructs a new File.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a File.
                 * @constructor
                 * @param {doramcap.storage.v1.File.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const File = function (properties) {
                    this.records = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * File index.
                 * @member {number|Long} index
                 * @memberof doramcap.storage.v1.File
                 * @instance
                 */
                File.prototype.index = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * File name.
                 * @member {string} name
                 * @memberof doramcap.storage.v1.File
                 * @instance
                 */
                File.prototype.name = "";

                /**
                 * File sealed.
                 * @member {boolean} sealed
                 * @memberof doramcap.storage.v1.File
                 * @instance
                 */
                File.prototype.sealed = false;

                /**
                 * File size.
                 * @member {number|Long} size
                 * @memberof doramcap.storage.v1.File
                 * @instance
                 */
                File.prototype.size = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * File parsedThrough.
                 * @member {number|Long} parsedThrough
                 * @memberof doramcap.storage.v1.File
                 * @instance
                 */
                File.prototype.parsedThrough = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * File records.
                 * @member {Array.<doramcap.storage.v1.Record.$Properties>} records
                 * @memberof doramcap.storage.v1.File
                 * @instance
                 */
                File.prototype.records = $util.emptyArray;

                /**
                 * File tailOffset.
                 * @member {number|Long} tailOffset
                 * @memberof doramcap.storage.v1.File
                 * @instance
                 */
                File.prototype.tailOffset = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * File tail.
                 * @member {Uint8Array} tail
                 * @memberof doramcap.storage.v1.File
                 * @instance
                 */
                File.prototype.tail = $util.newBuffer([]);

                /**
                 * Encodes the specified File message. Does not implicitly {@link doramcap.storage.v1.File.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.File
                 * @static
                 * @param {doramcap.storage.v1.File.$Properties} message File message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                File.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.index != null && $Object.hasOwnProperty.call(message, "index") && (typeof message.index === "object" ? message.index.low || message.index.high : message.index !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.index);
                    if (message.name != null && $Object.hasOwnProperty.call(message, "name") && message.name !== "")
                        writer.uint32(/* id 2, wireType 2 =*/18).string(message.name);
                    if (message.sealed != null && $Object.hasOwnProperty.call(message, "sealed") && message.sealed !== false)
                        writer.uint32(/* id 3, wireType 0 =*/24).bool(message.sealed);
                    if (message.size != null && $Object.hasOwnProperty.call(message, "size") && (typeof message.size === "object" ? message.size.low || message.size.high : message.size !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.size);
                    if (message.parsedThrough != null && $Object.hasOwnProperty.call(message, "parsedThrough") && (typeof message.parsedThrough === "object" ? message.parsedThrough.low || message.parsedThrough.high : message.parsedThrough !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.parsedThrough);
                    if (message.records != null && message.records.length)
                        for (let i = 0; i < message.records.length; ++i)
                            $root.doramcap.storage.v1.Record.encode(message.records[i], writer.uint32(/* id 6, wireType 2 =*/50).fork(), _depth + 1).ldelim();
                    if (message.tailOffset != null && $Object.hasOwnProperty.call(message, "tailOffset") && (typeof message.tailOffset === "object" ? message.tailOffset.low || message.tailOffset.high : message.tailOffset !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.tailOffset);
                    if (message.tail != null && $Object.hasOwnProperty.call(message, "tail") && message.tail.length)
                        writer.uint32(/* id 8, wireType 2 =*/66).bytes(message.tail);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a File message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.File
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.File & doramcap.storage.v1.File.$Shape} File
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                File.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.File();
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
                                    message.index = value;
                                else
                                    delete message.index;
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
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.bool())
                                    message.sealed = value;
                                else
                                    delete message.sealed;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.size = value;
                                else
                                    delete message.size;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.parsedThrough = value;
                                else
                                    delete message.parsedThrough;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.records && message.records.length))
                                    message.records = [];
                                message.records.push($root.doramcap.storage.v1.Record.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.tailOffset = value;
                                else
                                    delete message.tailOffset;
                                continue;
                            }
                        case 8: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.bytes()).length)
                                    message.tail = value;
                                else
                                    delete message.tail;
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
                 * Creates a File message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.File
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.File} File
                 */
                File.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.File)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.File: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.File();
                    if (object.index != null)
                        if (typeof object.index === "object" ? object.index.low || object.index.high : $Number(object.index) !== 0)
                            if ($util.Long)
                                message.index = $util.Long.fromValue(object.index, true);
                            else if (typeof object.index === "string")
                                message.index = $parseInt(object.index, 10);
                            else if (typeof object.index === "number")
                                message.index = object.index;
                            else if (typeof object.index === "object")
                                message.index = new $util.LongBits(object.index.low >>> 0, object.index.high >>> 0).toNumber(true);
                    if (object.name != null)
                        if (typeof object.name !== "string" || object.name.length)
                            message.name = $String(object.name);
                    if (object.sealed != null)
                        if (object.sealed)
                            message.sealed = $Boolean(object.sealed);
                    if (object.size != null)
                        if (typeof object.size === "object" ? object.size.low || object.size.high : $Number(object.size) !== 0)
                            if ($util.Long)
                                message.size = $util.Long.fromValue(object.size, true);
                            else if (typeof object.size === "string")
                                message.size = $parseInt(object.size, 10);
                            else if (typeof object.size === "number")
                                message.size = object.size;
                            else if (typeof object.size === "object")
                                message.size = new $util.LongBits(object.size.low >>> 0, object.size.high >>> 0).toNumber(true);
                    if (object.parsedThrough != null)
                        if (typeof object.parsedThrough === "object" ? object.parsedThrough.low || object.parsedThrough.high : $Number(object.parsedThrough) !== 0)
                            if ($util.Long)
                                message.parsedThrough = $util.Long.fromValue(object.parsedThrough, true);
                            else if (typeof object.parsedThrough === "string")
                                message.parsedThrough = $parseInt(object.parsedThrough, 10);
                            else if (typeof object.parsedThrough === "number")
                                message.parsedThrough = object.parsedThrough;
                            else if (typeof object.parsedThrough === "object")
                                message.parsedThrough = new $util.LongBits(object.parsedThrough.low >>> 0, object.parsedThrough.high >>> 0).toNumber(true);
                    if (object.records) {
                        if (!$Array.isArray(object.records))
                            throw $TypeError(".doramcap.storage.v1.File.records: array expected");
                        message.records = $Array(object.records.length);
                        for (let i = 0; i < object.records.length; ++i) {
                            if (!$util.isObject(object.records[i]))
                                throw $TypeError(".doramcap.storage.v1.File.records: object expected");
                            message.records[i] = $root.doramcap.storage.v1.Record.fromObject(object.records[i], _depth + 1);
                        }
                    }
                    if (object.tailOffset != null)
                        if (typeof object.tailOffset === "object" ? object.tailOffset.low || object.tailOffset.high : $Number(object.tailOffset) !== 0)
                            if ($util.Long)
                                message.tailOffset = $util.Long.fromValue(object.tailOffset, true);
                            else if (typeof object.tailOffset === "string")
                                message.tailOffset = $parseInt(object.tailOffset, 10);
                            else if (typeof object.tailOffset === "number")
                                message.tailOffset = object.tailOffset;
                            else if (typeof object.tailOffset === "object")
                                message.tailOffset = new $util.LongBits(object.tailOffset.low >>> 0, object.tailOffset.high >>> 0).toNumber(true);
                    if (object.tail != null)
                        if (object.tail.length)
                            if (typeof object.tail === "string")
                                $util.base64.decode(object.tail, message.tail = $util.newBuffer($util.base64.length(object.tail)), 0);
                            else if (object.tail.length >= 0)
                                message.tail = object.tail;
                    return message;
                };

                /**
                 * Creates a plain object from a File message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.File
                 * @static
                 * @param {doramcap.storage.v1.File} message File
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                File.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.arrays || options.defaults)
                        object.records = [];
                    if (options.defaults) {
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.index = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.index = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.name = "";
                        object.sealed = false;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.size = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.size = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.parsedThrough = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.parsedThrough = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.tailOffset = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.tailOffset = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if (options.bytes === $String)
                            object.tail = "";
                        else {
                            object.tail = [];
                            if (options.bytes !== $Array)
                                object.tail = $util.newBuffer(object.tail);
                        }
                    }
                    if (message.index != null && $Object.hasOwnProperty.call(message, "index"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.index = typeof message.index === "number" ? $BigInt(message.index) : $util.Long.fromBits(message.index.low >>> 0, message.index.high >>> 0, true).toBigInt();
                        else if (typeof message.index === "number")
                            object.index = options.longs === $String ? $String(message.index) : message.index;
                        else
                            object.index = options.longs === $String ? $util.Long.prototype.toString.call(message.index) : options.longs === $Number ? new $util.LongBits(message.index.low >>> 0, message.index.high >>> 0).toNumber(true) : message.index;
                    if (message.name != null && $Object.hasOwnProperty.call(message, "name"))
                        object.name = message.name;
                    if (message.sealed != null && $Object.hasOwnProperty.call(message, "sealed"))
                        object.sealed = message.sealed;
                    if (message.size != null && $Object.hasOwnProperty.call(message, "size"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.size = typeof message.size === "number" ? $BigInt(message.size) : $util.Long.fromBits(message.size.low >>> 0, message.size.high >>> 0, true).toBigInt();
                        else if (typeof message.size === "number")
                            object.size = options.longs === $String ? $String(message.size) : message.size;
                        else
                            object.size = options.longs === $String ? $util.Long.prototype.toString.call(message.size) : options.longs === $Number ? new $util.LongBits(message.size.low >>> 0, message.size.high >>> 0).toNumber(true) : message.size;
                    if (message.parsedThrough != null && $Object.hasOwnProperty.call(message, "parsedThrough"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.parsedThrough = typeof message.parsedThrough === "number" ? $BigInt(message.parsedThrough) : $util.Long.fromBits(message.parsedThrough.low >>> 0, message.parsedThrough.high >>> 0, true).toBigInt();
                        else if (typeof message.parsedThrough === "number")
                            object.parsedThrough = options.longs === $String ? $String(message.parsedThrough) : message.parsedThrough;
                        else
                            object.parsedThrough = options.longs === $String ? $util.Long.prototype.toString.call(message.parsedThrough) : options.longs === $Number ? new $util.LongBits(message.parsedThrough.low >>> 0, message.parsedThrough.high >>> 0).toNumber(true) : message.parsedThrough;
                    if (message.records && message.records.length) {
                        object.records = $Array(message.records.length);
                        for (let j = 0; j < message.records.length; ++j)
                            object.records[j] = $root.doramcap.storage.v1.Record.toObject(message.records[j], options, _depth + 1);
                    }
                    if (message.tailOffset != null && $Object.hasOwnProperty.call(message, "tailOffset"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.tailOffset = typeof message.tailOffset === "number" ? $BigInt(message.tailOffset) : $util.Long.fromBits(message.tailOffset.low >>> 0, message.tailOffset.high >>> 0, true).toBigInt();
                        else if (typeof message.tailOffset === "number")
                            object.tailOffset = options.longs === $String ? $String(message.tailOffset) : message.tailOffset;
                        else
                            object.tailOffset = options.longs === $String ? $util.Long.prototype.toString.call(message.tailOffset) : options.longs === $Number ? new $util.LongBits(message.tailOffset.low >>> 0, message.tailOffset.high >>> 0).toNumber(true) : message.tailOffset;
                    if (message.tail != null && $Object.hasOwnProperty.call(message, "tail"))
                        object.tail = options.bytes === $String ? $util.base64.encode(message.tail, 0, message.tail.length) : options.bytes === $Array ? $Array.prototype.slice.call(message.tail) : message.tail;
                    return object;
                };

                /**
                 * Converts this File to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.File
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                File.prototype.toJSON = function() {
                    return File.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for File
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.File
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                File.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.File";
                };

                return File;
            })();

            v1.Record = (function() {

                /**
                 * Properties of a Record.
                 * @typedef {Object} doramcap.storage.v1.Record.$Properties
                 * @property {number|Long|null} [offset] Record offset
                 * @property {number|Long|null} [len] Record len
                 * @property {string|null} [kind] Record kind
                 * @property {number|null} [channel] Record channel
                 * @property {number|null} [sequence] Record sequence
                 * @property {number|Long|null} [payload] Record payload
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Record.
                 * @memberof doramcap.storage.v1
                 * @interface IRecord
                 * @augments doramcap.storage.v1.Record.$Properties
                 * @deprecated Use doramcap.storage.v1.Record.$Properties instead.
                 */

                /**
                 * Shape of a Record.
                 * @typedef {doramcap.storage.v1.Record.$Properties} doramcap.storage.v1.Record.$Shape
                 */

                /**
                 * Constructs a new Record.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a Record.
                 * @constructor
                 * @param {doramcap.storage.v1.Record.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Record = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Record offset.
                 * @member {number|Long} offset
                 * @memberof doramcap.storage.v1.Record
                 * @instance
                 */
                Record.prototype.offset = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Record len.
                 * @member {number|Long} len
                 * @memberof doramcap.storage.v1.Record
                 * @instance
                 */
                Record.prototype.len = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Record kind.
                 * @member {string} kind
                 * @memberof doramcap.storage.v1.Record
                 * @instance
                 */
                Record.prototype.kind = "";

                /**
                 * Record channel.
                 * @member {number|null|undefined} channel
                 * @memberof doramcap.storage.v1.Record
                 * @instance
                 */
                Record.prototype.channel = null;

                /**
                 * Record sequence.
                 * @member {number|null|undefined} sequence
                 * @memberof doramcap.storage.v1.Record
                 * @instance
                 */
                Record.prototype.sequence = null;

                /**
                 * Record payload.
                 * @member {number|Long|null|undefined} payload
                 * @memberof doramcap.storage.v1.Record
                 * @instance
                 */
                Record.prototype.payload = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Record.prototype, "_channel", {
                    get: $util.oneOfGetter($oneOfFields = ["channel"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Record.prototype, "_sequence", {
                    get: $util.oneOfGetter($oneOfFields = ["sequence"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Record.prototype, "_payload", {
                    get: $util.oneOfGetter($oneOfFields = ["payload"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Record message. Does not implicitly {@link doramcap.storage.v1.Record.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Record
                 * @static
                 * @param {doramcap.storage.v1.Record.$Properties} message Record message or plain object to encode
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
                    if (message.offset != null && $Object.hasOwnProperty.call(message, "offset") && (typeof message.offset === "object" ? message.offset.low || message.offset.high : message.offset !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.offset);
                    if (message.len != null && $Object.hasOwnProperty.call(message, "len") && (typeof message.len === "object" ? message.len.low || message.len.high : message.len !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.len);
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind") && message.kind !== "")
                        writer.uint32(/* id 3, wireType 2 =*/26).string(message.kind);
                    if (message.channel != null && $Object.hasOwnProperty.call(message, "channel"))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.channel);
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence"))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint32(message.sequence);
                    if (message.payload != null && $Object.hasOwnProperty.call(message, "payload"))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.payload);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Record message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Record
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Record & doramcap.storage.v1.Record.$Shape} Record
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
                    message = _target || new $root.doramcap.storage.v1.Record();
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
                                    message.offset = value;
                                else
                                    delete message.offset;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.len = value;
                                else
                                    delete message.len;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.kind = value;
                                else
                                    delete message.kind;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                message.channel = reader.uint32();
                                message._channel = "channel";
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                message.sequence = reader.uint32();
                                message._sequence = "sequence";
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                message.payload = reader.uint64();
                                message._payload = "payload";
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
                 * @memberof doramcap.storage.v1.Record
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Record} Record
                 */
                Record.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Record)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Record: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Record();
                    if (object.offset != null)
                        if (typeof object.offset === "object" ? object.offset.low || object.offset.high : $Number(object.offset) !== 0)
                            if ($util.Long)
                                message.offset = $util.Long.fromValue(object.offset, true);
                            else if (typeof object.offset === "string")
                                message.offset = $parseInt(object.offset, 10);
                            else if (typeof object.offset === "number")
                                message.offset = object.offset;
                            else if (typeof object.offset === "object")
                                message.offset = new $util.LongBits(object.offset.low >>> 0, object.offset.high >>> 0).toNumber(true);
                    if (object.len != null)
                        if (typeof object.len === "object" ? object.len.low || object.len.high : $Number(object.len) !== 0)
                            if ($util.Long)
                                message.len = $util.Long.fromValue(object.len, true);
                            else if (typeof object.len === "string")
                                message.len = $parseInt(object.len, 10);
                            else if (typeof object.len === "number")
                                message.len = object.len;
                            else if (typeof object.len === "object")
                                message.len = new $util.LongBits(object.len.low >>> 0, object.len.high >>> 0).toNumber(true);
                    if (object.kind != null)
                        if (typeof object.kind !== "string" || object.kind.length)
                            message.kind = $String(object.kind);
                    if (object.channel != null)
                        message.channel = object.channel >>> 0;
                    if (object.sequence != null)
                        message.sequence = object.sequence >>> 0;
                    if (object.payload != null)
                        if ($util.Long)
                            message.payload = $util.Long.fromValue(object.payload, true);
                        else if (typeof object.payload === "string")
                            message.payload = $parseInt(object.payload, 10);
                        else if (typeof object.payload === "number")
                            message.payload = object.payload;
                        else if (typeof object.payload === "object")
                            message.payload = new $util.LongBits(object.payload.low >>> 0, object.payload.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a Record message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Record
                 * @static
                 * @param {doramcap.storage.v1.Record} message Record
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
                    if (options.defaults) {
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.offset = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.offset = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.len = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.len = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.kind = "";
                    }
                    if (message.offset != null && $Object.hasOwnProperty.call(message, "offset"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.offset = typeof message.offset === "number" ? $BigInt(message.offset) : $util.Long.fromBits(message.offset.low >>> 0, message.offset.high >>> 0, true).toBigInt();
                        else if (typeof message.offset === "number")
                            object.offset = options.longs === $String ? $String(message.offset) : message.offset;
                        else
                            object.offset = options.longs === $String ? $util.Long.prototype.toString.call(message.offset) : options.longs === $Number ? new $util.LongBits(message.offset.low >>> 0, message.offset.high >>> 0).toNumber(true) : message.offset;
                    if (message.len != null && $Object.hasOwnProperty.call(message, "len"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.len = typeof message.len === "number" ? $BigInt(message.len) : $util.Long.fromBits(message.len.low >>> 0, message.len.high >>> 0, true).toBigInt();
                        else if (typeof message.len === "number")
                            object.len = options.longs === $String ? $String(message.len) : message.len;
                        else
                            object.len = options.longs === $String ? $util.Long.prototype.toString.call(message.len) : options.longs === $Number ? new $util.LongBits(message.len.low >>> 0, message.len.high >>> 0).toNumber(true) : message.len;
                    if (message.kind != null && $Object.hasOwnProperty.call(message, "kind"))
                        object.kind = message.kind;
                    if (message.channel != null && $Object.hasOwnProperty.call(message, "channel"))
                        object.channel = message.channel;
                    if (message.sequence != null && $Object.hasOwnProperty.call(message, "sequence"))
                        object.sequence = message.sequence;
                    if (message.payload != null && $Object.hasOwnProperty.call(message, "payload"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.payload = typeof message.payload === "number" ? $BigInt(message.payload) : $util.Long.fromBits(message.payload.low >>> 0, message.payload.high >>> 0, true).toBigInt();
                        else if (typeof message.payload === "number")
                            object.payload = options.longs === $String ? $String(message.payload) : message.payload;
                        else
                            object.payload = options.longs === $String ? $util.Long.prototype.toString.call(message.payload) : options.longs === $Number ? new $util.LongBits(message.payload.low >>> 0, message.payload.high >>> 0).toNumber(true) : message.payload;
                    return object;
                };

                /**
                 * Converts this Record to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Record
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Record.prototype.toJSON = function() {
                    return Record.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Record
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Record
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Record.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Record";
                };

                return Record;
            })();

            v1.Exercises = (function() {

                /**
                 * Properties of an Exercises.
                 * @typedef {Object} doramcap.storage.v1.Exercises.$Properties
                 * @property {Array.<doramcap.storage.v1.PathReceipt.$Properties>|null} [receipts] Exercises receipts
                 * @property {number|Long|null} [heldReaders] Exercises heldReaders
                 * @property {doramcap.storage.v1.SlabPool.$Properties|null} [pool] Exercises pool
                 * @property {number|Long|null} [maxWriteBytes] Exercises maxWriteBytes
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of an Exercises.
                 * @memberof doramcap.storage.v1
                 * @interface IExercises
                 * @augments doramcap.storage.v1.Exercises.$Properties
                 * @deprecated Use doramcap.storage.v1.Exercises.$Properties instead.
                 */

                /**
                 * Shape of an Exercises.
                 * @typedef {doramcap.storage.v1.Exercises.$Properties} doramcap.storage.v1.Exercises.$Shape
                 */

                /**
                 * Constructs a new Exercises.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents an Exercises.
                 * @constructor
                 * @param {doramcap.storage.v1.Exercises.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Exercises = function (properties) {
                    this.receipts = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Exercises receipts.
                 * @member {Array.<doramcap.storage.v1.PathReceipt.$Properties>} receipts
                 * @memberof doramcap.storage.v1.Exercises
                 * @instance
                 */
                Exercises.prototype.receipts = $util.emptyArray;

                /**
                 * Exercises heldReaders.
                 * @member {number|Long} heldReaders
                 * @memberof doramcap.storage.v1.Exercises
                 * @instance
                 */
                Exercises.prototype.heldReaders = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Exercises pool.
                 * @member {doramcap.storage.v1.SlabPool.$Properties|null|undefined} pool
                 * @memberof doramcap.storage.v1.Exercises
                 * @instance
                 */
                Exercises.prototype.pool = null;

                /**
                 * Exercises maxWriteBytes.
                 * @member {number|Long} maxWriteBytes
                 * @memberof doramcap.storage.v1.Exercises
                 * @instance
                 */
                Exercises.prototype.maxWriteBytes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Encodes the specified Exercises message. Does not implicitly {@link doramcap.storage.v1.Exercises.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Exercises
                 * @static
                 * @param {doramcap.storage.v1.Exercises.$Properties} message Exercises message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Exercises.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.receipts != null && message.receipts.length)
                        for (let i = 0; i < message.receipts.length; ++i)
                            $root.doramcap.storage.v1.PathReceipt.encode(message.receipts[i], writer.uint32(/* id 1, wireType 2 =*/10).fork(), _depth + 1).ldelim();
                    if (message.heldReaders != null && $Object.hasOwnProperty.call(message, "heldReaders") && (typeof message.heldReaders === "object" ? message.heldReaders.low || message.heldReaders.high : message.heldReaders !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.heldReaders);
                    if (message.pool != null && $Object.hasOwnProperty.call(message, "pool"))
                        $root.doramcap.storage.v1.SlabPool.encode(message.pool, writer.uint32(/* id 3, wireType 2 =*/26).fork(), _depth + 1).ldelim();
                    if (message.maxWriteBytes != null && $Object.hasOwnProperty.call(message, "maxWriteBytes") && (typeof message.maxWriteBytes === "object" ? message.maxWriteBytes.low || message.maxWriteBytes.high : message.maxWriteBytes !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.maxWriteBytes);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes an Exercises message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Exercises
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Exercises & doramcap.storage.v1.Exercises.$Shape} Exercises
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Exercises.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Exercises();
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
                                if (!(message.receipts && message.receipts.length))
                                    message.receipts = [];
                                message.receipts.push($root.doramcap.storage.v1.PathReceipt.decode(reader, reader.size(), $undefined, _depth + 1));
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.heldReaders = value;
                                else
                                    delete message.heldReaders;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                message.pool = $root.doramcap.storage.v1.SlabPool.decode(reader, reader.size(), $undefined, _depth + 1, message.pool);
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.maxWriteBytes = value;
                                else
                                    delete message.maxWriteBytes;
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
                 * Creates an Exercises message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Exercises
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Exercises} Exercises
                 */
                Exercises.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Exercises)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Exercises: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Exercises();
                    if (object.receipts) {
                        if (!$Array.isArray(object.receipts))
                            throw $TypeError(".doramcap.storage.v1.Exercises.receipts: array expected");
                        message.receipts = $Array(object.receipts.length);
                        for (let i = 0; i < object.receipts.length; ++i) {
                            if (!$util.isObject(object.receipts[i]))
                                throw $TypeError(".doramcap.storage.v1.Exercises.receipts: object expected");
                            message.receipts[i] = $root.doramcap.storage.v1.PathReceipt.fromObject(object.receipts[i], _depth + 1);
                        }
                    }
                    if (object.heldReaders != null)
                        if (typeof object.heldReaders === "object" ? object.heldReaders.low || object.heldReaders.high : $Number(object.heldReaders) !== 0)
                            if ($util.Long)
                                message.heldReaders = $util.Long.fromValue(object.heldReaders, true);
                            else if (typeof object.heldReaders === "string")
                                message.heldReaders = $parseInt(object.heldReaders, 10);
                            else if (typeof object.heldReaders === "number")
                                message.heldReaders = object.heldReaders;
                            else if (typeof object.heldReaders === "object")
                                message.heldReaders = new $util.LongBits(object.heldReaders.low >>> 0, object.heldReaders.high >>> 0).toNumber(true);
                    if (object.pool != null) {
                        if (!$util.isObject(object.pool))
                            throw $TypeError(".doramcap.storage.v1.Exercises.pool: object expected");
                        message.pool = $root.doramcap.storage.v1.SlabPool.fromObject(object.pool, _depth + 1);
                    }
                    if (object.maxWriteBytes != null)
                        if (typeof object.maxWriteBytes === "object" ? object.maxWriteBytes.low || object.maxWriteBytes.high : $Number(object.maxWriteBytes) !== 0)
                            if ($util.Long)
                                message.maxWriteBytes = $util.Long.fromValue(object.maxWriteBytes, true);
                            else if (typeof object.maxWriteBytes === "string")
                                message.maxWriteBytes = $parseInt(object.maxWriteBytes, 10);
                            else if (typeof object.maxWriteBytes === "number")
                                message.maxWriteBytes = object.maxWriteBytes;
                            else if (typeof object.maxWriteBytes === "object")
                                message.maxWriteBytes = new $util.LongBits(object.maxWriteBytes.low >>> 0, object.maxWriteBytes.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from an Exercises message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Exercises
                 * @static
                 * @param {doramcap.storage.v1.Exercises} message Exercises
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Exercises.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.arrays || options.defaults)
                        object.receipts = [];
                    if (options.defaults) {
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.heldReaders = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.heldReaders = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.pool = null;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.maxWriteBytes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.maxWriteBytes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.receipts && message.receipts.length) {
                        object.receipts = $Array(message.receipts.length);
                        for (let j = 0; j < message.receipts.length; ++j)
                            object.receipts[j] = $root.doramcap.storage.v1.PathReceipt.toObject(message.receipts[j], options, _depth + 1);
                    }
                    if (message.heldReaders != null && $Object.hasOwnProperty.call(message, "heldReaders"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.heldReaders = typeof message.heldReaders === "number" ? $BigInt(message.heldReaders) : $util.Long.fromBits(message.heldReaders.low >>> 0, message.heldReaders.high >>> 0, true).toBigInt();
                        else if (typeof message.heldReaders === "number")
                            object.heldReaders = options.longs === $String ? $String(message.heldReaders) : message.heldReaders;
                        else
                            object.heldReaders = options.longs === $String ? $util.Long.prototype.toString.call(message.heldReaders) : options.longs === $Number ? new $util.LongBits(message.heldReaders.low >>> 0, message.heldReaders.high >>> 0).toNumber(true) : message.heldReaders;
                    if (message.pool != null && $Object.hasOwnProperty.call(message, "pool"))
                        object.pool = $root.doramcap.storage.v1.SlabPool.toObject(message.pool, options, _depth + 1);
                    if (message.maxWriteBytes != null && $Object.hasOwnProperty.call(message, "maxWriteBytes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.maxWriteBytes = typeof message.maxWriteBytes === "number" ? $BigInt(message.maxWriteBytes) : $util.Long.fromBits(message.maxWriteBytes.low >>> 0, message.maxWriteBytes.high >>> 0, true).toBigInt();
                        else if (typeof message.maxWriteBytes === "number")
                            object.maxWriteBytes = options.longs === $String ? $String(message.maxWriteBytes) : message.maxWriteBytes;
                        else
                            object.maxWriteBytes = options.longs === $String ? $util.Long.prototype.toString.call(message.maxWriteBytes) : options.longs === $Number ? new $util.LongBits(message.maxWriteBytes.low >>> 0, message.maxWriteBytes.high >>> 0).toNumber(true) : message.maxWriteBytes;
                    return object;
                };

                /**
                 * Converts this Exercises to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Exercises
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Exercises.prototype.toJSON = function() {
                    return Exercises.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Exercises
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Exercises
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Exercises.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Exercises";
                };

                return Exercises;
            })();

            v1.SlabPool = (function() {

                /**
                 * Properties of a SlabPool.
                 * @typedef {Object} doramcap.storage.v1.SlabPool.$Properties
                 * @property {number|Long|null} [id] SlabPool id
                 * @property {number|Long|null} [available] SlabPool available
                 * @property {number|Long|null} [capacity] SlabPool capacity
                 * @property {Array.<doramcap.storage.v1.Slab.$Properties>|null} [slabs] SlabPool slabs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a SlabPool.
                 * @memberof doramcap.storage.v1
                 * @interface ISlabPool
                 * @augments doramcap.storage.v1.SlabPool.$Properties
                 * @deprecated Use doramcap.storage.v1.SlabPool.$Properties instead.
                 */

                /**
                 * Shape of a SlabPool.
                 * @typedef {doramcap.storage.v1.SlabPool.$Properties} doramcap.storage.v1.SlabPool.$Shape
                 */

                /**
                 * Constructs a new SlabPool.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a SlabPool.
                 * @constructor
                 * @param {doramcap.storage.v1.SlabPool.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const SlabPool = function (properties) {
                    this.slabs = [];
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * SlabPool id.
                 * @member {number|Long} id
                 * @memberof doramcap.storage.v1.SlabPool
                 * @instance
                 */
                SlabPool.prototype.id = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * SlabPool available.
                 * @member {number|Long} available
                 * @memberof doramcap.storage.v1.SlabPool
                 * @instance
                 */
                SlabPool.prototype.available = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * SlabPool capacity.
                 * @member {number|Long} capacity
                 * @memberof doramcap.storage.v1.SlabPool
                 * @instance
                 */
                SlabPool.prototype.capacity = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * SlabPool slabs.
                 * @member {Array.<doramcap.storage.v1.Slab.$Properties>} slabs
                 * @memberof doramcap.storage.v1.SlabPool
                 * @instance
                 */
                SlabPool.prototype.slabs = $util.emptyArray;

                /**
                 * Encodes the specified SlabPool message. Does not implicitly {@link doramcap.storage.v1.SlabPool.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.SlabPool
                 * @static
                 * @param {doramcap.storage.v1.SlabPool.$Properties} message SlabPool message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                SlabPool.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.id != null && $Object.hasOwnProperty.call(message, "id") && (typeof message.id === "object" ? message.id.low || message.id.high : message.id !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.id);
                    if (message.available != null && $Object.hasOwnProperty.call(message, "available") && (typeof message.available === "object" ? message.available.low || message.available.high : message.available !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.available);
                    if (message.capacity != null && $Object.hasOwnProperty.call(message, "capacity") && (typeof message.capacity === "object" ? message.capacity.low || message.capacity.high : message.capacity !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.capacity);
                    if (message.slabs != null && message.slabs.length)
                        for (let i = 0; i < message.slabs.length; ++i)
                            $root.doramcap.storage.v1.Slab.encode(message.slabs[i], writer.uint32(/* id 4, wireType 2 =*/34).fork(), _depth + 1).ldelim();
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a SlabPool message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.SlabPool
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.SlabPool & doramcap.storage.v1.SlabPool.$Shape} SlabPool
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                SlabPool.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.SlabPool();
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
                                    message.available = value;
                                else
                                    delete message.available;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.capacity = value;
                                else
                                    delete message.capacity;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 2)
                                    break;
                                if (!(message.slabs && message.slabs.length))
                                    message.slabs = [];
                                message.slabs.push($root.doramcap.storage.v1.Slab.decode(reader, reader.size(), $undefined, _depth + 1));
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
                 * Creates a SlabPool message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.SlabPool
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.SlabPool} SlabPool
                 */
                SlabPool.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.SlabPool)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.SlabPool: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.SlabPool();
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
                    if (object.available != null)
                        if (typeof object.available === "object" ? object.available.low || object.available.high : $Number(object.available) !== 0)
                            if ($util.Long)
                                message.available = $util.Long.fromValue(object.available, true);
                            else if (typeof object.available === "string")
                                message.available = $parseInt(object.available, 10);
                            else if (typeof object.available === "number")
                                message.available = object.available;
                            else if (typeof object.available === "object")
                                message.available = new $util.LongBits(object.available.low >>> 0, object.available.high >>> 0).toNumber(true);
                    if (object.capacity != null)
                        if (typeof object.capacity === "object" ? object.capacity.low || object.capacity.high : $Number(object.capacity) !== 0)
                            if ($util.Long)
                                message.capacity = $util.Long.fromValue(object.capacity, true);
                            else if (typeof object.capacity === "string")
                                message.capacity = $parseInt(object.capacity, 10);
                            else if (typeof object.capacity === "number")
                                message.capacity = object.capacity;
                            else if (typeof object.capacity === "object")
                                message.capacity = new $util.LongBits(object.capacity.low >>> 0, object.capacity.high >>> 0).toNumber(true);
                    if (object.slabs) {
                        if (!$Array.isArray(object.slabs))
                            throw $TypeError(".doramcap.storage.v1.SlabPool.slabs: array expected");
                        message.slabs = $Array(object.slabs.length);
                        for (let i = 0; i < object.slabs.length; ++i) {
                            if (!$util.isObject(object.slabs[i]))
                                throw $TypeError(".doramcap.storage.v1.SlabPool.slabs: object expected");
                            message.slabs[i] = $root.doramcap.storage.v1.Slab.fromObject(object.slabs[i], _depth + 1);
                        }
                    }
                    return message;
                };

                /**
                 * Creates a plain object from a SlabPool message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.SlabPool
                 * @static
                 * @param {doramcap.storage.v1.SlabPool} message SlabPool
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                SlabPool.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.arrays || options.defaults)
                        object.slabs = [];
                    if (options.defaults) {
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.id = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.id = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.available = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.available = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.capacity = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.capacity = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                    }
                    if (message.id != null && $Object.hasOwnProperty.call(message, "id"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.id = typeof message.id === "number" ? $BigInt(message.id) : $util.Long.fromBits(message.id.low >>> 0, message.id.high >>> 0, true).toBigInt();
                        else if (typeof message.id === "number")
                            object.id = options.longs === $String ? $String(message.id) : message.id;
                        else
                            object.id = options.longs === $String ? $util.Long.prototype.toString.call(message.id) : options.longs === $Number ? new $util.LongBits(message.id.low >>> 0, message.id.high >>> 0).toNumber(true) : message.id;
                    if (message.available != null && $Object.hasOwnProperty.call(message, "available"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.available = typeof message.available === "number" ? $BigInt(message.available) : $util.Long.fromBits(message.available.low >>> 0, message.available.high >>> 0, true).toBigInt();
                        else if (typeof message.available === "number")
                            object.available = options.longs === $String ? $String(message.available) : message.available;
                        else
                            object.available = options.longs === $String ? $util.Long.prototype.toString.call(message.available) : options.longs === $Number ? new $util.LongBits(message.available.low >>> 0, message.available.high >>> 0).toNumber(true) : message.available;
                    if (message.capacity != null && $Object.hasOwnProperty.call(message, "capacity"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.capacity = typeof message.capacity === "number" ? $BigInt(message.capacity) : $util.Long.fromBits(message.capacity.low >>> 0, message.capacity.high >>> 0, true).toBigInt();
                        else if (typeof message.capacity === "number")
                            object.capacity = options.longs === $String ? $String(message.capacity) : message.capacity;
                        else
                            object.capacity = options.longs === $String ? $util.Long.prototype.toString.call(message.capacity) : options.longs === $Number ? new $util.LongBits(message.capacity.low >>> 0, message.capacity.high >>> 0).toNumber(true) : message.capacity;
                    if (message.slabs && message.slabs.length) {
                        object.slabs = $Array(message.slabs.length);
                        for (let j = 0; j < message.slabs.length; ++j)
                            object.slabs[j] = $root.doramcap.storage.v1.Slab.toObject(message.slabs[j], options, _depth + 1);
                    }
                    return object;
                };

                /**
                 * Converts this SlabPool to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.SlabPool
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                SlabPool.prototype.toJSON = function() {
                    return SlabPool.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for SlabPool
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.SlabPool
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                SlabPool.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.SlabPool";
                };

                return SlabPool;
            })();

            v1.Slab = (function() {

                /**
                 * Properties of a Slab.
                 * @typedef {Object} doramcap.storage.v1.Slab.$Properties
                 * @property {number|Long|null} [region] Slab region
                 * @property {number|Long|null} [use] Slab use
                 * @property {string|null} [phase] Slab phase
                 * @property {number|Long|null} [readers] Slab readers
                 * @property {number|Long|null} [ioLeases] Slab ioLeases
                 * @property {boolean|null} [retainedAfterIo] Slab retainedAfterIo
                 * @property {number|Long|null} [ageUs] Slab ageUs
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a Slab.
                 * @memberof doramcap.storage.v1
                 * @interface ISlab
                 * @augments doramcap.storage.v1.Slab.$Properties
                 * @deprecated Use doramcap.storage.v1.Slab.$Properties instead.
                 */

                /**
                 * Shape of a Slab.
                 * @typedef {doramcap.storage.v1.Slab.$Properties} doramcap.storage.v1.Slab.$Shape
                 */

                /**
                 * Constructs a new Slab.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a Slab.
                 * @constructor
                 * @param {doramcap.storage.v1.Slab.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const Slab = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * Slab region.
                 * @member {number|Long} region
                 * @memberof doramcap.storage.v1.Slab
                 * @instance
                 */
                Slab.prototype.region = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Slab use.
                 * @member {number|Long} use
                 * @memberof doramcap.storage.v1.Slab
                 * @instance
                 */
                Slab.prototype.use = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Slab phase.
                 * @member {string} phase
                 * @memberof doramcap.storage.v1.Slab
                 * @instance
                 */
                Slab.prototype.phase = "";

                /**
                 * Slab readers.
                 * @member {number|Long} readers
                 * @memberof doramcap.storage.v1.Slab
                 * @instance
                 */
                Slab.prototype.readers = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Slab ioLeases.
                 * @member {number|Long} ioLeases
                 * @memberof doramcap.storage.v1.Slab
                 * @instance
                 */
                Slab.prototype.ioLeases = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * Slab retainedAfterIo.
                 * @member {boolean} retainedAfterIo
                 * @memberof doramcap.storage.v1.Slab
                 * @instance
                 */
                Slab.prototype.retainedAfterIo = false;

                /**
                 * Slab ageUs.
                 * @member {number|Long|null|undefined} ageUs
                 * @memberof doramcap.storage.v1.Slab
                 * @instance
                 */
                Slab.prototype.ageUs = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(Slab.prototype, "_ageUs", {
                    get: $util.oneOfGetter($oneOfFields = ["ageUs"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified Slab message. Does not implicitly {@link doramcap.storage.v1.Slab.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.Slab
                 * @static
                 * @param {doramcap.storage.v1.Slab.$Properties} message Slab message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                Slab.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.region != null && $Object.hasOwnProperty.call(message, "region") && (typeof message.region === "object" ? message.region.low || message.region.high : message.region !== 0))
                        writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.region);
                    if (message.use != null && $Object.hasOwnProperty.call(message, "use") && (typeof message.use === "object" ? message.use.low || message.use.high : message.use !== 0))
                        writer.uint32(/* id 2, wireType 0 =*/16).uint64(message.use);
                    if (message.phase != null && $Object.hasOwnProperty.call(message, "phase") && message.phase !== "")
                        writer.uint32(/* id 3, wireType 2 =*/26).string(message.phase);
                    if (message.readers != null && $Object.hasOwnProperty.call(message, "readers") && (typeof message.readers === "object" ? message.readers.low || message.readers.high : message.readers !== 0))
                        writer.uint32(/* id 4, wireType 0 =*/32).uint64(message.readers);
                    if (message.ioLeases != null && $Object.hasOwnProperty.call(message, "ioLeases") && (typeof message.ioLeases === "object" ? message.ioLeases.low || message.ioLeases.high : message.ioLeases !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.ioLeases);
                    if (message.retainedAfterIo != null && $Object.hasOwnProperty.call(message, "retainedAfterIo") && message.retainedAfterIo !== false)
                        writer.uint32(/* id 6, wireType 0 =*/48).bool(message.retainedAfterIo);
                    if (message.ageUs != null && $Object.hasOwnProperty.call(message, "ageUs"))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.ageUs);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a Slab message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.Slab
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Slab & doramcap.storage.v1.Slab.$Shape} Slab
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                Slab.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.Slab();
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
                                    message.region = value;
                                else
                                    delete message.region;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.use = value;
                                else
                                    delete message.use;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.phase = value;
                                else
                                    delete message.phase;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.readers = value;
                                else
                                    delete message.readers;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.ioLeases = value;
                                else
                                    delete message.ioLeases;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.bool())
                                    message.retainedAfterIo = value;
                                else
                                    delete message.retainedAfterIo;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                message.ageUs = reader.uint64();
                                message._ageUs = "ageUs";
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
                 * Creates a Slab message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.Slab
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.Slab} Slab
                 */
                Slab.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.Slab)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.Slab: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.Slab();
                    if (object.region != null)
                        if (typeof object.region === "object" ? object.region.low || object.region.high : $Number(object.region) !== 0)
                            if ($util.Long)
                                message.region = $util.Long.fromValue(object.region, true);
                            else if (typeof object.region === "string")
                                message.region = $parseInt(object.region, 10);
                            else if (typeof object.region === "number")
                                message.region = object.region;
                            else if (typeof object.region === "object")
                                message.region = new $util.LongBits(object.region.low >>> 0, object.region.high >>> 0).toNumber(true);
                    if (object.use != null)
                        if (typeof object.use === "object" ? object.use.low || object.use.high : $Number(object.use) !== 0)
                            if ($util.Long)
                                message.use = $util.Long.fromValue(object.use, true);
                            else if (typeof object.use === "string")
                                message.use = $parseInt(object.use, 10);
                            else if (typeof object.use === "number")
                                message.use = object.use;
                            else if (typeof object.use === "object")
                                message.use = new $util.LongBits(object.use.low >>> 0, object.use.high >>> 0).toNumber(true);
                    if (object.phase != null)
                        if (typeof object.phase !== "string" || object.phase.length)
                            message.phase = $String(object.phase);
                    if (object.readers != null)
                        if (typeof object.readers === "object" ? object.readers.low || object.readers.high : $Number(object.readers) !== 0)
                            if ($util.Long)
                                message.readers = $util.Long.fromValue(object.readers, true);
                            else if (typeof object.readers === "string")
                                message.readers = $parseInt(object.readers, 10);
                            else if (typeof object.readers === "number")
                                message.readers = object.readers;
                            else if (typeof object.readers === "object")
                                message.readers = new $util.LongBits(object.readers.low >>> 0, object.readers.high >>> 0).toNumber(true);
                    if (object.ioLeases != null)
                        if (typeof object.ioLeases === "object" ? object.ioLeases.low || object.ioLeases.high : $Number(object.ioLeases) !== 0)
                            if ($util.Long)
                                message.ioLeases = $util.Long.fromValue(object.ioLeases, true);
                            else if (typeof object.ioLeases === "string")
                                message.ioLeases = $parseInt(object.ioLeases, 10);
                            else if (typeof object.ioLeases === "number")
                                message.ioLeases = object.ioLeases;
                            else if (typeof object.ioLeases === "object")
                                message.ioLeases = new $util.LongBits(object.ioLeases.low >>> 0, object.ioLeases.high >>> 0).toNumber(true);
                    if (object.retainedAfterIo != null)
                        if (object.retainedAfterIo)
                            message.retainedAfterIo = $Boolean(object.retainedAfterIo);
                    if (object.ageUs != null)
                        if ($util.Long)
                            message.ageUs = $util.Long.fromValue(object.ageUs, true);
                        else if (typeof object.ageUs === "string")
                            message.ageUs = $parseInt(object.ageUs, 10);
                        else if (typeof object.ageUs === "number")
                            message.ageUs = object.ageUs;
                        else if (typeof object.ageUs === "object")
                            message.ageUs = new $util.LongBits(object.ageUs.low >>> 0, object.ageUs.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a Slab message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.Slab
                 * @static
                 * @param {doramcap.storage.v1.Slab} message Slab
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                Slab.toObject = function (message, options, _depth) {
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
                            object.region = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.region = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.use = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.use = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.phase = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.readers = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.readers = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.ioLeases = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.ioLeases = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.retainedAfterIo = false;
                    }
                    if (message.region != null && $Object.hasOwnProperty.call(message, "region"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.region = typeof message.region === "number" ? $BigInt(message.region) : $util.Long.fromBits(message.region.low >>> 0, message.region.high >>> 0, true).toBigInt();
                        else if (typeof message.region === "number")
                            object.region = options.longs === $String ? $String(message.region) : message.region;
                        else
                            object.region = options.longs === $String ? $util.Long.prototype.toString.call(message.region) : options.longs === $Number ? new $util.LongBits(message.region.low >>> 0, message.region.high >>> 0).toNumber(true) : message.region;
                    if (message.use != null && $Object.hasOwnProperty.call(message, "use"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.use = typeof message.use === "number" ? $BigInt(message.use) : $util.Long.fromBits(message.use.low >>> 0, message.use.high >>> 0, true).toBigInt();
                        else if (typeof message.use === "number")
                            object.use = options.longs === $String ? $String(message.use) : message.use;
                        else
                            object.use = options.longs === $String ? $util.Long.prototype.toString.call(message.use) : options.longs === $Number ? new $util.LongBits(message.use.low >>> 0, message.use.high >>> 0).toNumber(true) : message.use;
                    if (message.phase != null && $Object.hasOwnProperty.call(message, "phase"))
                        object.phase = message.phase;
                    if (message.readers != null && $Object.hasOwnProperty.call(message, "readers"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.readers = typeof message.readers === "number" ? $BigInt(message.readers) : $util.Long.fromBits(message.readers.low >>> 0, message.readers.high >>> 0, true).toBigInt();
                        else if (typeof message.readers === "number")
                            object.readers = options.longs === $String ? $String(message.readers) : message.readers;
                        else
                            object.readers = options.longs === $String ? $util.Long.prototype.toString.call(message.readers) : options.longs === $Number ? new $util.LongBits(message.readers.low >>> 0, message.readers.high >>> 0).toNumber(true) : message.readers;
                    if (message.ioLeases != null && $Object.hasOwnProperty.call(message, "ioLeases"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.ioLeases = typeof message.ioLeases === "number" ? $BigInt(message.ioLeases) : $util.Long.fromBits(message.ioLeases.low >>> 0, message.ioLeases.high >>> 0, true).toBigInt();
                        else if (typeof message.ioLeases === "number")
                            object.ioLeases = options.longs === $String ? $String(message.ioLeases) : message.ioLeases;
                        else
                            object.ioLeases = options.longs === $String ? $util.Long.prototype.toString.call(message.ioLeases) : options.longs === $Number ? new $util.LongBits(message.ioLeases.low >>> 0, message.ioLeases.high >>> 0).toNumber(true) : message.ioLeases;
                    if (message.retainedAfterIo != null && $Object.hasOwnProperty.call(message, "retainedAfterIo"))
                        object.retainedAfterIo = message.retainedAfterIo;
                    if (message.ageUs != null && $Object.hasOwnProperty.call(message, "ageUs"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.ageUs = typeof message.ageUs === "number" ? $BigInt(message.ageUs) : $util.Long.fromBits(message.ageUs.low >>> 0, message.ageUs.high >>> 0, true).toBigInt();
                        else if (typeof message.ageUs === "number")
                            object.ageUs = options.longs === $String ? $String(message.ageUs) : message.ageUs;
                        else
                            object.ageUs = options.longs === $String ? $util.Long.prototype.toString.call(message.ageUs) : options.longs === $Number ? new $util.LongBits(message.ageUs.low >>> 0, message.ageUs.high >>> 0).toNumber(true) : message.ageUs;
                    return object;
                };

                /**
                 * Converts this Slab to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.Slab
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                Slab.prototype.toJSON = function() {
                    return Slab.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for Slab
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.Slab
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                Slab.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.Slab";
                };

                return Slab;
            })();

            v1.PathReceipt = (function() {

                /**
                 * Properties of a PathReceipt.
                 * @typedef {Object} doramcap.storage.v1.PathReceipt.$Properties
                 * @property {string|null} [status] PathReceipt status
                 * @property {string|null} [error] PathReceipt error
                 * @property {number|Long|null} [id] PathReceipt id
                 * @property {string|null} [action] PathReceipt action
                 * @property {number|Long|null} [registrationUpdates] PathReceipt registrationUpdates
                 * @property {number|Long|null} [messages] PathReceipt messages
                 * @property {number|Long|null} [retired] PathReceipt retired
                 * @property {number|Long|null} [bytes] PathReceipt bytes
                 * @property {number|null} [first] PathReceipt first
                 * @property {number|null} [last] PathReceipt last
                 * @property {number|Long|null} [write] PathReceipt write
                 * @property {number|Long|null} [region] PathReceipt region
                 * @property {number|Long|null} [slabUse] PathReceipt slabUse
                 * @property {number|Long|null} [offset] PathReceipt offset
                 * @property {number|Long|null} [length] PathReceipt length
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */

                /**
                 * Properties of a PathReceipt.
                 * @memberof doramcap.storage.v1
                 * @interface IPathReceipt
                 * @augments doramcap.storage.v1.PathReceipt.$Properties
                 * @deprecated Use doramcap.storage.v1.PathReceipt.$Properties instead.
                 */

                /**
                 * Shape of a PathReceipt.
                 * @typedef {doramcap.storage.v1.PathReceipt.$Properties} doramcap.storage.v1.PathReceipt.$Shape
                 */

                /**
                 * Constructs a new PathReceipt.
                 * @memberof doramcap.storage.v1
                 * @classdesc Represents a PathReceipt.
                 * @constructor
                 * @param {doramcap.storage.v1.PathReceipt.$Properties=} [properties] Properties to set
                 * @property {Array.<Uint8Array>} [$unknowns] Unknown fields preserved while decoding when enabled
                 */
                const PathReceipt = function (properties) {
                    if (properties)
                        for (let keys = $Object.keys(properties), i = 0; i < keys.length; ++i)
                            if (properties[keys[i]] != null && keys[i] !== "__proto__")
                                this[keys[i]] = properties[keys[i]];
                };

                /**
                 * PathReceipt status.
                 * @member {string} status
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.status = "";

                /**
                 * PathReceipt error.
                 * @member {string} error
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.error = "";

                /**
                 * PathReceipt id.
                 * @member {number|Long} id
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.id = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * PathReceipt action.
                 * @member {string} action
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.action = "";

                /**
                 * PathReceipt registrationUpdates.
                 * @member {number|Long} registrationUpdates
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.registrationUpdates = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * PathReceipt messages.
                 * @member {number|Long} messages
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.messages = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * PathReceipt retired.
                 * @member {number|Long} retired
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.retired = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * PathReceipt bytes.
                 * @member {number|Long} bytes
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.bytes = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                /**
                 * PathReceipt first.
                 * @member {number} first
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.first = 0;

                /**
                 * PathReceipt last.
                 * @member {number} last
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.last = 0;

                /**
                 * PathReceipt write.
                 * @member {number|Long|null|undefined} write
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.write = null;

                /**
                 * PathReceipt region.
                 * @member {number|Long|null|undefined} region
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.region = null;

                /**
                 * PathReceipt slabUse.
                 * @member {number|Long|null|undefined} slabUse
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.slabUse = null;

                /**
                 * PathReceipt offset.
                 * @member {number|Long|null|undefined} offset
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.offset = null;

                /**
                 * PathReceipt length.
                 * @member {number|Long|null|undefined} length
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 */
                PathReceipt.prototype.length = null;

                // OneOf field names bound to virtual getters and setters
                let $oneOfFields;

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(PathReceipt.prototype, "_write", {
                    get: $util.oneOfGetter($oneOfFields = ["write"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(PathReceipt.prototype, "_region", {
                    get: $util.oneOfGetter($oneOfFields = ["region"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(PathReceipt.prototype, "_slabUse", {
                    get: $util.oneOfGetter($oneOfFields = ["slabUse"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(PathReceipt.prototype, "_offset", {
                    get: $util.oneOfGetter($oneOfFields = ["offset"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                // Virtual OneOf for proto3 optional field
                $Object.defineProperty(PathReceipt.prototype, "_length", {
                    get: $util.oneOfGetter($oneOfFields = ["length"]),
                    set: $util.oneOfSetter($oneOfFields)
                });

                /**
                 * Encodes the specified PathReceipt message. Does not implicitly {@link doramcap.storage.v1.PathReceipt.verify|verify} messages.
                 * @function encode
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @static
                 * @param {doramcap.storage.v1.PathReceipt.$Properties} message PathReceipt message or plain object to encode
                 * @param {$protobuf.Writer} [writer] Writer to encode to
                 * @returns {$protobuf.Writer} Writer
                 */
                PathReceipt.encode = function (message, writer, _depth) {
                    if (!writer)
                        writer = $Writer.create();
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    if (message.status != null && $Object.hasOwnProperty.call(message, "status") && message.status !== "")
                        writer.uint32(/* id 1, wireType 2 =*/10).string(message.status);
                    if (message.error != null && $Object.hasOwnProperty.call(message, "error") && message.error !== "")
                        writer.uint32(/* id 2, wireType 2 =*/18).string(message.error);
                    if (message.id != null && $Object.hasOwnProperty.call(message, "id") && (typeof message.id === "object" ? message.id.low || message.id.high : message.id !== 0))
                        writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.id);
                    if (message.action != null && $Object.hasOwnProperty.call(message, "action") && message.action !== "")
                        writer.uint32(/* id 4, wireType 2 =*/34).string(message.action);
                    if (message.registrationUpdates != null && $Object.hasOwnProperty.call(message, "registrationUpdates") && (typeof message.registrationUpdates === "object" ? message.registrationUpdates.low || message.registrationUpdates.high : message.registrationUpdates !== 0))
                        writer.uint32(/* id 5, wireType 0 =*/40).uint64(message.registrationUpdates);
                    if (message.messages != null && $Object.hasOwnProperty.call(message, "messages") && (typeof message.messages === "object" ? message.messages.low || message.messages.high : message.messages !== 0))
                        writer.uint32(/* id 6, wireType 0 =*/48).uint64(message.messages);
                    if (message.retired != null && $Object.hasOwnProperty.call(message, "retired") && (typeof message.retired === "object" ? message.retired.low || message.retired.high : message.retired !== 0))
                        writer.uint32(/* id 7, wireType 0 =*/56).uint64(message.retired);
                    if (message.bytes != null && $Object.hasOwnProperty.call(message, "bytes") && (typeof message.bytes === "object" ? message.bytes.low || message.bytes.high : message.bytes !== 0))
                        writer.uint32(/* id 8, wireType 0 =*/64).uint64(message.bytes);
                    if (message.first != null && $Object.hasOwnProperty.call(message, "first") && message.first !== 0)
                        writer.uint32(/* id 9, wireType 0 =*/72).uint32(message.first);
                    if (message.last != null && $Object.hasOwnProperty.call(message, "last") && message.last !== 0)
                        writer.uint32(/* id 10, wireType 0 =*/80).uint32(message.last);
                    if (message.write != null && $Object.hasOwnProperty.call(message, "write"))
                        writer.uint32(/* id 11, wireType 0 =*/88).uint64(message.write);
                    if (message.region != null && $Object.hasOwnProperty.call(message, "region"))
                        writer.uint32(/* id 12, wireType 0 =*/96).uint64(message.region);
                    if (message.slabUse != null && $Object.hasOwnProperty.call(message, "slabUse"))
                        writer.uint32(/* id 13, wireType 0 =*/104).uint64(message.slabUse);
                    if (message.offset != null && $Object.hasOwnProperty.call(message, "offset"))
                        writer.uint32(/* id 14, wireType 0 =*/112).uint64(message.offset);
                    if (message.length != null && $Object.hasOwnProperty.call(message, "length"))
                        writer.uint32(/* id 15, wireType 0 =*/120).uint64(message.length);
                    if (message.$unknowns != null && $Object.hasOwnProperty.call(message, "$unknowns"))
                        for (let i = 0; i < message.$unknowns.length; ++i)
                            writer.raw(message.$unknowns[i]);
                    return writer;
                };

                /**
                 * Decodes a PathReceipt message from the specified reader or buffer.
                 * @function decode
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @static
                 * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                 * @param {number} [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.PathReceipt & doramcap.storage.v1.PathReceipt.$Shape} PathReceipt
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                PathReceipt.decode = function (reader, length, _end, _depth, _target) {
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
                    message = _target || new $root.doramcap.storage.v1.PathReceipt();
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
                                    message.status = value;
                                else
                                    delete message.status;
                                continue;
                            }
                        case 2: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.error = value;
                                else
                                    delete message.error;
                                continue;
                            }
                        case 3: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.id = value;
                                else
                                    delete message.id;
                                continue;
                            }
                        case 4: {
                                if (wireType !== 2)
                                    break;
                                if ((value = reader.stringVerify()).length)
                                    message.action = value;
                                else
                                    delete message.action;
                                continue;
                            }
                        case 5: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.registrationUpdates = value;
                                else
                                    delete message.registrationUpdates;
                                continue;
                            }
                        case 6: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.messages = value;
                                else
                                    delete message.messages;
                                continue;
                            }
                        case 7: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.retired = value;
                                else
                                    delete message.retired;
                                continue;
                            }
                        case 8: {
                                if (wireType !== 0)
                                    break;
                                if (typeof (value = reader.uint64()) === "object" ? value.low || value.high : value !== 0)
                                    message.bytes = value;
                                else
                                    delete message.bytes;
                                continue;
                            }
                        case 9: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.first = value;
                                else
                                    delete message.first;
                                continue;
                            }
                        case 10: {
                                if (wireType !== 0)
                                    break;
                                if (value = reader.uint32())
                                    message.last = value;
                                else
                                    delete message.last;
                                continue;
                            }
                        case 11: {
                                if (wireType !== 0)
                                    break;
                                message.write = reader.uint64();
                                message._write = "write";
                                continue;
                            }
                        case 12: {
                                if (wireType !== 0)
                                    break;
                                message.region = reader.uint64();
                                message._region = "region";
                                continue;
                            }
                        case 13: {
                                if (wireType !== 0)
                                    break;
                                message.slabUse = reader.uint64();
                                message._slabUse = "slabUse";
                                continue;
                            }
                        case 14: {
                                if (wireType !== 0)
                                    break;
                                message.offset = reader.uint64();
                                message._offset = "offset";
                                continue;
                            }
                        case 15: {
                                if (wireType !== 0)
                                    break;
                                message.length = reader.uint64();
                                message._length = "length";
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
                 * Creates a PathReceipt message from a plain object. Also converts values to their respective internal types.
                 * @function fromObject
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @static
                 * @param {Object.<string,*>} object Plain object
                 * @returns {doramcap.storage.v1.PathReceipt} PathReceipt
                 */
                PathReceipt.fromObject = function (object, _depth) {
                    if (object instanceof $root.doramcap.storage.v1.PathReceipt)
                        return object;
                    if (!$util.isObject(object))
                        throw $TypeError(".doramcap.storage.v1.PathReceipt: object expected");
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let message = new $root.doramcap.storage.v1.PathReceipt();
                    if (object.status != null)
                        if (typeof object.status !== "string" || object.status.length)
                            message.status = $String(object.status);
                    if (object.error != null)
                        if (typeof object.error !== "string" || object.error.length)
                            message.error = $String(object.error);
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
                    if (object.action != null)
                        if (typeof object.action !== "string" || object.action.length)
                            message.action = $String(object.action);
                    if (object.registrationUpdates != null)
                        if (typeof object.registrationUpdates === "object" ? object.registrationUpdates.low || object.registrationUpdates.high : $Number(object.registrationUpdates) !== 0)
                            if ($util.Long)
                                message.registrationUpdates = $util.Long.fromValue(object.registrationUpdates, true);
                            else if (typeof object.registrationUpdates === "string")
                                message.registrationUpdates = $parseInt(object.registrationUpdates, 10);
                            else if (typeof object.registrationUpdates === "number")
                                message.registrationUpdates = object.registrationUpdates;
                            else if (typeof object.registrationUpdates === "object")
                                message.registrationUpdates = new $util.LongBits(object.registrationUpdates.low >>> 0, object.registrationUpdates.high >>> 0).toNumber(true);
                    if (object.messages != null)
                        if (typeof object.messages === "object" ? object.messages.low || object.messages.high : $Number(object.messages) !== 0)
                            if ($util.Long)
                                message.messages = $util.Long.fromValue(object.messages, true);
                            else if (typeof object.messages === "string")
                                message.messages = $parseInt(object.messages, 10);
                            else if (typeof object.messages === "number")
                                message.messages = object.messages;
                            else if (typeof object.messages === "object")
                                message.messages = new $util.LongBits(object.messages.low >>> 0, object.messages.high >>> 0).toNumber(true);
                    if (object.retired != null)
                        if (typeof object.retired === "object" ? object.retired.low || object.retired.high : $Number(object.retired) !== 0)
                            if ($util.Long)
                                message.retired = $util.Long.fromValue(object.retired, true);
                            else if (typeof object.retired === "string")
                                message.retired = $parseInt(object.retired, 10);
                            else if (typeof object.retired === "number")
                                message.retired = object.retired;
                            else if (typeof object.retired === "object")
                                message.retired = new $util.LongBits(object.retired.low >>> 0, object.retired.high >>> 0).toNumber(true);
                    if (object.bytes != null)
                        if (typeof object.bytes === "object" ? object.bytes.low || object.bytes.high : $Number(object.bytes) !== 0)
                            if ($util.Long)
                                message.bytes = $util.Long.fromValue(object.bytes, true);
                            else if (typeof object.bytes === "string")
                                message.bytes = $parseInt(object.bytes, 10);
                            else if (typeof object.bytes === "number")
                                message.bytes = object.bytes;
                            else if (typeof object.bytes === "object")
                                message.bytes = new $util.LongBits(object.bytes.low >>> 0, object.bytes.high >>> 0).toNumber(true);
                    if (object.first != null)
                        if ($Number(object.first) !== 0)
                            message.first = object.first >>> 0;
                    if (object.last != null)
                        if ($Number(object.last) !== 0)
                            message.last = object.last >>> 0;
                    if (object.write != null)
                        if ($util.Long)
                            message.write = $util.Long.fromValue(object.write, true);
                        else if (typeof object.write === "string")
                            message.write = $parseInt(object.write, 10);
                        else if (typeof object.write === "number")
                            message.write = object.write;
                        else if (typeof object.write === "object")
                            message.write = new $util.LongBits(object.write.low >>> 0, object.write.high >>> 0).toNumber(true);
                    if (object.region != null)
                        if ($util.Long)
                            message.region = $util.Long.fromValue(object.region, true);
                        else if (typeof object.region === "string")
                            message.region = $parseInt(object.region, 10);
                        else if (typeof object.region === "number")
                            message.region = object.region;
                        else if (typeof object.region === "object")
                            message.region = new $util.LongBits(object.region.low >>> 0, object.region.high >>> 0).toNumber(true);
                    if (object.slabUse != null)
                        if ($util.Long)
                            message.slabUse = $util.Long.fromValue(object.slabUse, true);
                        else if (typeof object.slabUse === "string")
                            message.slabUse = $parseInt(object.slabUse, 10);
                        else if (typeof object.slabUse === "number")
                            message.slabUse = object.slabUse;
                        else if (typeof object.slabUse === "object")
                            message.slabUse = new $util.LongBits(object.slabUse.low >>> 0, object.slabUse.high >>> 0).toNumber(true);
                    if (object.offset != null)
                        if ($util.Long)
                            message.offset = $util.Long.fromValue(object.offset, true);
                        else if (typeof object.offset === "string")
                            message.offset = $parseInt(object.offset, 10);
                        else if (typeof object.offset === "number")
                            message.offset = object.offset;
                        else if (typeof object.offset === "object")
                            message.offset = new $util.LongBits(object.offset.low >>> 0, object.offset.high >>> 0).toNumber(true);
                    if (object.length != null)
                        if ($util.Long)
                            message.length = $util.Long.fromValue(object.length, true);
                        else if (typeof object.length === "string")
                            message.length = $parseInt(object.length, 10);
                        else if (typeof object.length === "number")
                            message.length = object.length;
                        else if (typeof object.length === "object")
                            message.length = new $util.LongBits(object.length.low >>> 0, object.length.high >>> 0).toNumber(true);
                    return message;
                };

                /**
                 * Creates a plain object from a PathReceipt message. Also converts values to other types if specified.
                 * @function toObject
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @static
                 * @param {doramcap.storage.v1.PathReceipt} message PathReceipt
                 * @param {$protobuf.IConversionOptions} [options] Conversion options
                 * @returns {Object.<string,*>} Plain object
                 */
                PathReceipt.toObject = function (message, options, _depth) {
                    if (!options)
                        options = {};
                    if (_depth === $undefined)
                        _depth = 0;
                    if (_depth > $util.recursionLimit)
                        throw $Error("max depth exceeded");
                    let object = {};
                    if (options.defaults) {
                        object.status = "";
                        object.error = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.id = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.id = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.action = "";
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.registrationUpdates = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.registrationUpdates = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.messages = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.messages = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.retired = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.retired = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        if ($util.Long) {
                            let long = new $util.Long(0, 0, true);
                            object.bytes = options.longs === $String ? long.toString() : options.longs === $Number ? long.toNumber() : typeof $BigInt !== "undefined" && options.longs === $BigInt ? long.toBigInt() : long;
                        } else
                            object.bytes = options.longs === $String ? "0" : typeof $BigInt !== "undefined" && options.longs === $BigInt ? $BigInt("0") : 0;
                        object.first = 0;
                        object.last = 0;
                    }
                    if (message.status != null && $Object.hasOwnProperty.call(message, "status"))
                        object.status = message.status;
                    if (message.error != null && $Object.hasOwnProperty.call(message, "error"))
                        object.error = message.error;
                    if (message.id != null && $Object.hasOwnProperty.call(message, "id"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.id = typeof message.id === "number" ? $BigInt(message.id) : $util.Long.fromBits(message.id.low >>> 0, message.id.high >>> 0, true).toBigInt();
                        else if (typeof message.id === "number")
                            object.id = options.longs === $String ? $String(message.id) : message.id;
                        else
                            object.id = options.longs === $String ? $util.Long.prototype.toString.call(message.id) : options.longs === $Number ? new $util.LongBits(message.id.low >>> 0, message.id.high >>> 0).toNumber(true) : message.id;
                    if (message.action != null && $Object.hasOwnProperty.call(message, "action"))
                        object.action = message.action;
                    if (message.registrationUpdates != null && $Object.hasOwnProperty.call(message, "registrationUpdates"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.registrationUpdates = typeof message.registrationUpdates === "number" ? $BigInt(message.registrationUpdates) : $util.Long.fromBits(message.registrationUpdates.low >>> 0, message.registrationUpdates.high >>> 0, true).toBigInt();
                        else if (typeof message.registrationUpdates === "number")
                            object.registrationUpdates = options.longs === $String ? $String(message.registrationUpdates) : message.registrationUpdates;
                        else
                            object.registrationUpdates = options.longs === $String ? $util.Long.prototype.toString.call(message.registrationUpdates) : options.longs === $Number ? new $util.LongBits(message.registrationUpdates.low >>> 0, message.registrationUpdates.high >>> 0).toNumber(true) : message.registrationUpdates;
                    if (message.messages != null && $Object.hasOwnProperty.call(message, "messages"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.messages = typeof message.messages === "number" ? $BigInt(message.messages) : $util.Long.fromBits(message.messages.low >>> 0, message.messages.high >>> 0, true).toBigInt();
                        else if (typeof message.messages === "number")
                            object.messages = options.longs === $String ? $String(message.messages) : message.messages;
                        else
                            object.messages = options.longs === $String ? $util.Long.prototype.toString.call(message.messages) : options.longs === $Number ? new $util.LongBits(message.messages.low >>> 0, message.messages.high >>> 0).toNumber(true) : message.messages;
                    if (message.retired != null && $Object.hasOwnProperty.call(message, "retired"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.retired = typeof message.retired === "number" ? $BigInt(message.retired) : $util.Long.fromBits(message.retired.low >>> 0, message.retired.high >>> 0, true).toBigInt();
                        else if (typeof message.retired === "number")
                            object.retired = options.longs === $String ? $String(message.retired) : message.retired;
                        else
                            object.retired = options.longs === $String ? $util.Long.prototype.toString.call(message.retired) : options.longs === $Number ? new $util.LongBits(message.retired.low >>> 0, message.retired.high >>> 0).toNumber(true) : message.retired;
                    if (message.bytes != null && $Object.hasOwnProperty.call(message, "bytes"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.bytes = typeof message.bytes === "number" ? $BigInt(message.bytes) : $util.Long.fromBits(message.bytes.low >>> 0, message.bytes.high >>> 0, true).toBigInt();
                        else if (typeof message.bytes === "number")
                            object.bytes = options.longs === $String ? $String(message.bytes) : message.bytes;
                        else
                            object.bytes = options.longs === $String ? $util.Long.prototype.toString.call(message.bytes) : options.longs === $Number ? new $util.LongBits(message.bytes.low >>> 0, message.bytes.high >>> 0).toNumber(true) : message.bytes;
                    if (message.first != null && $Object.hasOwnProperty.call(message, "first"))
                        object.first = message.first;
                    if (message.last != null && $Object.hasOwnProperty.call(message, "last"))
                        object.last = message.last;
                    if (message.write != null && $Object.hasOwnProperty.call(message, "write"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.write = typeof message.write === "number" ? $BigInt(message.write) : $util.Long.fromBits(message.write.low >>> 0, message.write.high >>> 0, true).toBigInt();
                        else if (typeof message.write === "number")
                            object.write = options.longs === $String ? $String(message.write) : message.write;
                        else
                            object.write = options.longs === $String ? $util.Long.prototype.toString.call(message.write) : options.longs === $Number ? new $util.LongBits(message.write.low >>> 0, message.write.high >>> 0).toNumber(true) : message.write;
                    if (message.region != null && $Object.hasOwnProperty.call(message, "region"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.region = typeof message.region === "number" ? $BigInt(message.region) : $util.Long.fromBits(message.region.low >>> 0, message.region.high >>> 0, true).toBigInt();
                        else if (typeof message.region === "number")
                            object.region = options.longs === $String ? $String(message.region) : message.region;
                        else
                            object.region = options.longs === $String ? $util.Long.prototype.toString.call(message.region) : options.longs === $Number ? new $util.LongBits(message.region.low >>> 0, message.region.high >>> 0).toNumber(true) : message.region;
                    if (message.slabUse != null && $Object.hasOwnProperty.call(message, "slabUse"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.slabUse = typeof message.slabUse === "number" ? $BigInt(message.slabUse) : $util.Long.fromBits(message.slabUse.low >>> 0, message.slabUse.high >>> 0, true).toBigInt();
                        else if (typeof message.slabUse === "number")
                            object.slabUse = options.longs === $String ? $String(message.slabUse) : message.slabUse;
                        else
                            object.slabUse = options.longs === $String ? $util.Long.prototype.toString.call(message.slabUse) : options.longs === $Number ? new $util.LongBits(message.slabUse.low >>> 0, message.slabUse.high >>> 0).toNumber(true) : message.slabUse;
                    if (message.offset != null && $Object.hasOwnProperty.call(message, "offset"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.offset = typeof message.offset === "number" ? $BigInt(message.offset) : $util.Long.fromBits(message.offset.low >>> 0, message.offset.high >>> 0, true).toBigInt();
                        else if (typeof message.offset === "number")
                            object.offset = options.longs === $String ? $String(message.offset) : message.offset;
                        else
                            object.offset = options.longs === $String ? $util.Long.prototype.toString.call(message.offset) : options.longs === $Number ? new $util.LongBits(message.offset.low >>> 0, message.offset.high >>> 0).toNumber(true) : message.offset;
                    if (message.length != null && $Object.hasOwnProperty.call(message, "length"))
                        if (typeof $BigInt !== "undefined" && options.longs === $BigInt)
                            object.length = typeof message.length === "number" ? $BigInt(message.length) : $util.Long.fromBits(message.length.low >>> 0, message.length.high >>> 0, true).toBigInt();
                        else if (typeof message.length === "number")
                            object.length = options.longs === $String ? $String(message.length) : message.length;
                        else
                            object.length = options.longs === $String ? $util.Long.prototype.toString.call(message.length) : options.longs === $Number ? new $util.LongBits(message.length.low >>> 0, message.length.high >>> 0).toNumber(true) : message.length;
                    return object;
                };

                /**
                 * Converts this PathReceipt to JSON.
                 * @function toJSON
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @instance
                 * @returns {Object.<string,*>} JSON object
                 */
                PathReceipt.prototype.toJSON = function() {
                    return PathReceipt.toObject(this, $protobuf.util.toJSONOptions);
                };

                /**
                 * Gets the type url for PathReceipt
                 * @function getTypeUrl
                 * @memberof doramcap.storage.v1.PathReceipt
                 * @static
                 * @param {string} [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns {string} The type url
                 */
                PathReceipt.getTypeUrl = function(prefix) {
                    if (prefix === $undefined)
                        prefix = "type.googleapis.com";
                    return prefix + "/doramcap.storage.v1.PathReceipt";
                };

                return PathReceipt;
            })();

            return v1;
        })();

        return storage;
    })();

    return doramcap;
})();

export {
  $root as default
};
