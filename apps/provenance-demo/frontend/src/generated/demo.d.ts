import * as $protobuf from "protobufjs";
import Long = require("long");

/** Namespace doramcap. */
export namespace doramcap {

    /** Namespace provenance_demo. */
    namespace provenance_demo {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of a Payload.
             * @deprecated Use doramcap.provenance_demo.v1.Payload.$Properties instead.
             */
            interface IPayload extends doramcap.provenance_demo.v1.Payload.$Properties {
            }

            /** Represents a Payload. */
            class Payload {

                /**
                 * Constructs a new Payload.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance_demo.v1.Payload.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Payload generation. */
                generation?: (doramcap.provenance.v1.GenerationRef.$Properties|null);

                /** Payload producedNs. */
                producedNs: (number|Long);

                /**
                 * Encodes the specified Payload message. Does not implicitly {@link doramcap.provenance_demo.v1.Payload.verify|verify} messages.
                 * @param message Payload message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance_demo.v1.Payload.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Payload message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance_demo.v1.Payload & doramcap.provenance_demo.v1.Payload.$Shape} Payload
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance_demo.v1.Payload & doramcap.provenance_demo.v1.Payload.$Shape;

                /**
                 * Creates a Payload message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Payload
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance_demo.v1.Payload;

                /**
                 * Creates a plain object from a Payload message. Also converts values to other types if specified.
                 * @param message Payload
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance_demo.v1.Payload, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Payload to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Payload
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Payload {

                /** Properties of a Payload. */
                interface $Properties {

                    /** Payload generation */
                    generation?: (doramcap.provenance.v1.GenerationRef.$Properties|null);

                    /** Payload producedNs */
                    producedNs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Payload. */
                type $Shape = doramcap.provenance_demo.v1.Payload.$Properties;
            }

            /**
             * Properties of a Clock.
             * @deprecated Use doramcap.provenance_demo.v1.Clock.$Properties instead.
             */
            interface IClock extends doramcap.provenance_demo.v1.Clock.$Properties {
            }

            /** Represents a Clock. */
            class Clock {

                /**
                 * Constructs a new Clock.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance_demo.v1.Clock.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Clock session. */
                session: (number|Long);

                /** Clock unixOriginNs. */
                unixOriginNs: (number|Long);

                /** Clock domain. */
                domain: string;

                /**
                 * Encodes the specified Clock message. Does not implicitly {@link doramcap.provenance_demo.v1.Clock.verify|verify} messages.
                 * @param message Clock message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance_demo.v1.Clock.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Clock message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance_demo.v1.Clock & doramcap.provenance_demo.v1.Clock.$Shape} Clock
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance_demo.v1.Clock & doramcap.provenance_demo.v1.Clock.$Shape;

                /**
                 * Creates a Clock message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Clock
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance_demo.v1.Clock;

                /**
                 * Creates a plain object from a Clock message. Also converts values to other types if specified.
                 * @param message Clock
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance_demo.v1.Clock, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Clock to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Clock
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Clock {

                /** Properties of a Clock. */
                interface $Properties {

                    /** Clock session */
                    session?: (number|Long|null);

                    /** Clock unixOriginNs */
                    unixOriginNs?: (number|Long|null);

                    /** Clock domain */
                    domain?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Clock. */
                type $Shape = doramcap.provenance_demo.v1.Clock.$Properties;
            }

            /**
             * Properties of a NodeStatus.
             * @deprecated Use doramcap.provenance_demo.v1.NodeStatus.$Properties instead.
             */
            interface INodeStatus extends doramcap.provenance_demo.v1.NodeStatus.$Properties {
            }

            /** Represents a NodeStatus. */
            class NodeStatus {

                /**
                 * Constructs a new NodeStatus.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance_demo.v1.NodeStatus.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** NodeStatus node. */
                node: string;

                /** NodeStatus pid. */
                pid: number;

                /** NodeStatus epoch. */
                epoch: string;

                /** NodeStatus activityQueueDrops. */
                activityQueueDrops: (number|Long);

                /** NodeStatus simulatedReportDrops. */
                simulatedReportDrops: (number|Long);

                /**
                 * Encodes the specified NodeStatus message. Does not implicitly {@link doramcap.provenance_demo.v1.NodeStatus.verify|verify} messages.
                 * @param message NodeStatus message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance_demo.v1.NodeStatus.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a NodeStatus message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance_demo.v1.NodeStatus & doramcap.provenance_demo.v1.NodeStatus.$Shape} NodeStatus
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance_demo.v1.NodeStatus & doramcap.provenance_demo.v1.NodeStatus.$Shape;

                /**
                 * Creates a NodeStatus message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns NodeStatus
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance_demo.v1.NodeStatus;

                /**
                 * Creates a plain object from a NodeStatus message. Also converts values to other types if specified.
                 * @param message NodeStatus
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance_demo.v1.NodeStatus, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this NodeStatus to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for NodeStatus
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace NodeStatus {

                /** Properties of a NodeStatus. */
                interface $Properties {

                    /** NodeStatus node */
                    node?: (string|null);

                    /** NodeStatus pid */
                    pid?: (number|null);

                    /** NodeStatus epoch */
                    epoch?: (string|null);

                    /** NodeStatus activityQueueDrops */
                    activityQueueDrops?: (number|Long|null);

                    /** NodeStatus simulatedReportDrops */
                    simulatedReportDrops?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a NodeStatus. */
                type $Shape = doramcap.provenance_demo.v1.NodeStatus.$Properties;
            }

            /**
             * Properties of a Gap.
             * @deprecated Use doramcap.provenance_demo.v1.Gap.$Properties instead.
             */
            interface IGap extends doramcap.provenance_demo.v1.Gap.$Properties {
            }

            /** Represents a Gap. */
            class Gap {

                /**
                 * Constructs a new Gap.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance_demo.v1.Gap.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Gap envelopes. */
                envelopes: (number|Long);

                /**
                 * Encodes the specified Gap message. Does not implicitly {@link doramcap.provenance_demo.v1.Gap.verify|verify} messages.
                 * @param message Gap message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance_demo.v1.Gap.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Gap message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance_demo.v1.Gap & doramcap.provenance_demo.v1.Gap.$Shape} Gap
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance_demo.v1.Gap & doramcap.provenance_demo.v1.Gap.$Shape;

                /**
                 * Creates a Gap message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Gap
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance_demo.v1.Gap;

                /**
                 * Creates a plain object from a Gap message. Also converts values to other types if specified.
                 * @param message Gap
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance_demo.v1.Gap, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Gap to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Gap
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Gap {

                /** Properties of a Gap. */
                interface $Properties {

                    /** Gap envelopes */
                    envelopes?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Gap. */
                type $Shape = doramcap.provenance_demo.v1.Gap.$Properties;
            }

            /**
             * Properties of an Envelope.
             * @deprecated Use doramcap.provenance_demo.v1.Envelope.$Properties instead.
             */
            interface IEnvelope extends doramcap.provenance_demo.v1.Envelope.$Properties {
            }

            /** Represents an Envelope. */
            class Envelope {

                /**
                 * Constructs a new Envelope.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance_demo.v1.Envelope.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Envelope version. */
                version: number;

                /** Envelope streamId. */
                streamId: string;

                /** Envelope sequence. */
                sequence: (number|Long);

                /** Envelope step. */
                step?: (doramcap.provenance.v1.StepUpdate.$Properties|null);

                /** Envelope result. */
                result?: (doramcap.provenance.v1.MeasurementResult.$Properties|null);

                /** Envelope activity. */
                activity?: (doramcap.activity.v1.Batch.$Properties|null);

                /** Envelope clock. */
                clock?: (doramcap.provenance_demo.v1.Clock.$Properties|null);

                /** Envelope status. */
                status?: (doramcap.provenance_demo.v1.NodeStatus.$Properties|null);

                /** Envelope gap. */
                gap?: (doramcap.provenance_demo.v1.Gap.$Properties|null);

                /** Envelope body. */
                body?: ("step"|"result"|"activity"|"clock"|"status"|"gap");

                /**
                 * Encodes the specified Envelope message. Does not implicitly {@link doramcap.provenance_demo.v1.Envelope.verify|verify} messages.
                 * @param message Envelope message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance_demo.v1.Envelope.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Envelope message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance_demo.v1.Envelope & doramcap.provenance_demo.v1.Envelope.$Shape} Envelope
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance_demo.v1.Envelope & doramcap.provenance_demo.v1.Envelope.$Shape;

                /**
                 * Creates an Envelope message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Envelope
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance_demo.v1.Envelope;

                /**
                 * Creates a plain object from an Envelope message. Also converts values to other types if specified.
                 * @param message Envelope
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance_demo.v1.Envelope, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Envelope to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Envelope
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Envelope {

                /** Properties of an Envelope. */
                interface $Properties {

                    /** Envelope version */
                    version?: (number|null);

                    /** Envelope streamId */
                    streamId?: (string|null);

                    /** Envelope sequence */
                    sequence?: (number|Long|null);

                    /** Envelope step */
                    step?: (doramcap.provenance.v1.StepUpdate.$Properties|null);

                    /** Envelope result */
                    result?: (doramcap.provenance.v1.MeasurementResult.$Properties|null);

                    /** Envelope activity */
                    activity?: (doramcap.activity.v1.Batch.$Properties|null);

                    /** Envelope clock */
                    clock?: (doramcap.provenance_demo.v1.Clock.$Properties|null);

                    /** Envelope status */
                    status?: (doramcap.provenance_demo.v1.NodeStatus.$Properties|null);

                    /** Envelope gap */
                    gap?: (doramcap.provenance_demo.v1.Gap.$Properties|null);

                    /** Envelope body */
                    body?: ("step"|"result"|"activity"|"clock"|"status"|"gap");

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Narrowed shape of an Envelope. */
                type $Shape = {
                  version?: number|null;
                  streamId?: string|null;
                  sequence?: number|Long|null;
                  step?: doramcap.provenance.v1.StepUpdate.$Shape|null;
                  result?: doramcap.provenance.v1.MeasurementResult.$Shape|null;
                  activity?: doramcap.activity.v1.Batch.$Shape|null;
                  clock?: doramcap.provenance_demo.v1.Clock.$Shape|null;
                  status?: doramcap.provenance_demo.v1.NodeStatus.$Shape|null;
                  gap?: doramcap.provenance_demo.v1.Gap.$Shape|null;
                  $unknowns?: Uint8Array[];
                } & (
                  ({ body?: undefined; step?: null; result?: null; activity?: null; clock?: null; status?: null; gap?: null }|{ body?: "step"; step: doramcap.provenance.v1.StepUpdate.$Shape; result?: null; activity?: null; clock?: null; status?: null; gap?: null }|{ body?: "result"; step?: null; result: doramcap.provenance.v1.MeasurementResult.$Shape; activity?: null; clock?: null; status?: null; gap?: null }|{ body?: "activity"; step?: null; result?: null; activity: doramcap.activity.v1.Batch.$Shape; clock?: null; status?: null; gap?: null }|{ body?: "clock"; step?: null; result?: null; activity?: null; clock: doramcap.provenance_demo.v1.Clock.$Shape; status?: null; gap?: null }|{ body?: "status"; step?: null; result?: null; activity?: null; clock?: null; status: doramcap.provenance_demo.v1.NodeStatus.$Shape; gap?: null }|{ body?: "gap"; step?: null; result?: null; activity?: null; clock?: null; status?: null; gap: doramcap.provenance_demo.v1.Gap.$Shape })
                );
            }
        }
    }

    /** Namespace provenance. */
    namespace provenance {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of a Step.
             * @deprecated Use doramcap.provenance.v1.Step.$Properties instead.
             */
            interface IStep extends doramcap.provenance.v1.Step.$Properties {
            }

            /** Represents a Step. */
            class Step {

                /**
                 * Constructs a new Step.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance.v1.Step.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Step producer. */
                producer: string;

                /** Step name. */
                name: string;

                /**
                 * Encodes the specified Step message. Does not implicitly {@link doramcap.provenance.v1.Step.verify|verify} messages.
                 * @param message Step message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance.v1.Step.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Step message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.Step & doramcap.provenance.v1.Step.$Shape} Step
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance.v1.Step & doramcap.provenance.v1.Step.$Shape;

                /**
                 * Creates a Step message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Step
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance.v1.Step;

                /**
                 * Creates a plain object from a Step message. Also converts values to other types if specified.
                 * @param message Step
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance.v1.Step, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Step to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Step
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Step {

                /** Properties of a Step. */
                interface $Properties {

                    /** Step producer */
                    producer?: (string|null);

                    /** Step name */
                    name?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Step. */
                type $Shape = doramcap.provenance.v1.Step.$Properties;
            }

            /**
             * Properties of a GenerationRef.
             * @deprecated Use doramcap.provenance.v1.GenerationRef.$Properties instead.
             */
            interface IGenerationRef extends doramcap.provenance.v1.GenerationRef.$Properties {
            }

            /** Represents a GenerationRef. */
            class GenerationRef {

                /**
                 * Constructs a new GenerationRef.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance.v1.GenerationRef.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** GenerationRef producer. */
                producer: string;

                /** GenerationRef epoch. */
                epoch: string;

                /** GenerationRef step. */
                step: string;

                /** GenerationRef generation. */
                generation: string;

                /** GenerationRef sequence. */
                sequence: (number|Long);

                /**
                 * Encodes the specified GenerationRef message. Does not implicitly {@link doramcap.provenance.v1.GenerationRef.verify|verify} messages.
                 * @param message GenerationRef message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance.v1.GenerationRef.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a GenerationRef message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.GenerationRef & doramcap.provenance.v1.GenerationRef.$Shape} GenerationRef
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance.v1.GenerationRef & doramcap.provenance.v1.GenerationRef.$Shape;

                /**
                 * Creates a GenerationRef message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns GenerationRef
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance.v1.GenerationRef;

                /**
                 * Creates a plain object from a GenerationRef message. Also converts values to other types if specified.
                 * @param message GenerationRef
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance.v1.GenerationRef, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this GenerationRef to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for GenerationRef
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace GenerationRef {

                /** Properties of a GenerationRef. */
                interface $Properties {

                    /** GenerationRef producer */
                    producer?: (string|null);

                    /** GenerationRef epoch */
                    epoch?: (string|null);

                    /** GenerationRef step */
                    step?: (string|null);

                    /** GenerationRef generation */
                    generation?: (string|null);

                    /** GenerationRef sequence */
                    sequence?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a GenerationRef. */
                type $Shape = doramcap.provenance.v1.GenerationRef.$Properties;
            }

            /**
             * Properties of an Inputs.
             * @deprecated Use doramcap.provenance.v1.Inputs.$Properties instead.
             */
            interface IInputs extends doramcap.provenance.v1.Inputs.$Properties {
            }

            /** Represents an Inputs. */
            class Inputs {

                /**
                 * Constructs a new Inputs.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance.v1.Inputs.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Inputs refs. */
                refs: doramcap.provenance.v1.GenerationRef.$Properties[];

                /**
                 * Encodes the specified Inputs message. Does not implicitly {@link doramcap.provenance.v1.Inputs.verify|verify} messages.
                 * @param message Inputs message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance.v1.Inputs.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Inputs message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.Inputs & doramcap.provenance.v1.Inputs.$Shape} Inputs
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance.v1.Inputs & doramcap.provenance.v1.Inputs.$Shape;

                /**
                 * Creates an Inputs message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Inputs
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance.v1.Inputs;

                /**
                 * Creates a plain object from an Inputs message. Also converts values to other types if specified.
                 * @param message Inputs
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance.v1.Inputs, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Inputs to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Inputs
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Inputs {

                /** Properties of an Inputs. */
                interface $Properties {

                    /** Inputs refs */
                    refs?: (doramcap.provenance.v1.GenerationRef.$Properties[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an Inputs. */
                type $Shape = doramcap.provenance.v1.Inputs.$Properties;
            }

            /**
             * Properties of a StepUpdate.
             * @deprecated Use doramcap.provenance.v1.StepUpdate.$Properties instead.
             */
            interface IStepUpdate extends doramcap.provenance.v1.StepUpdate.$Properties {
            }

            /** Represents a StepUpdate. */
            class StepUpdate {

                /**
                 * Constructs a new StepUpdate.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance.v1.StepUpdate.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** StepUpdate version. */
                version: number;

                /** StepUpdate generation. */
                generation?: (doramcap.provenance.v1.GenerationRef.$Properties|null);

                /** StepUpdate clockDomain. */
                clockDomain: string;

                /** StepUpdate invocation. */
                invocation: string;

                /** StepUpdate startNs. */
                startNs?: (number|Long|null);

                /** StepUpdate endNs. */
                endNs?: (number|Long|null);

                /** StepUpdate inputs. */
                inputs?: (doramcap.provenance.v1.Inputs.$Properties|null);

                /**
                 * Encodes the specified StepUpdate message. Does not implicitly {@link doramcap.provenance.v1.StepUpdate.verify|verify} messages.
                 * @param message StepUpdate message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance.v1.StepUpdate.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a StepUpdate message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.StepUpdate & doramcap.provenance.v1.StepUpdate.$Shape} StepUpdate
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance.v1.StepUpdate & doramcap.provenance.v1.StepUpdate.$Shape;

                /**
                 * Creates a StepUpdate message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns StepUpdate
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance.v1.StepUpdate;

                /**
                 * Creates a plain object from a StepUpdate message. Also converts values to other types if specified.
                 * @param message StepUpdate
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance.v1.StepUpdate, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this StepUpdate to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for StepUpdate
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace StepUpdate {

                /** Properties of a StepUpdate. */
                interface $Properties {

                    /** StepUpdate version */
                    version?: (number|null);

                    /** StepUpdate generation */
                    generation?: (doramcap.provenance.v1.GenerationRef.$Properties|null);

                    /** StepUpdate clockDomain */
                    clockDomain?: (string|null);

                    /** StepUpdate invocation */
                    invocation?: (string|null);

                    /** StepUpdate startNs */
                    startNs?: (number|Long|null);

                    /** StepUpdate endNs */
                    endNs?: (number|Long|null);

                    /** StepUpdate inputs */
                    inputs?: (doramcap.provenance.v1.Inputs.$Properties|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a StepUpdate. */
                type $Shape = doramcap.provenance.v1.StepUpdate.$Properties;
            }

            /** Boundary enum. */
            enum Boundary {

                /** BOUNDARY_UNSPECIFIED value */
                BOUNDARY_UNSPECIFIED = 0,

                /** START value */
                START = 1,

                /** END value */
                END = 2
            }

            /** Selector enum. */
            enum Selector {

                /** SELECTOR_UNSPECIFIED value */
                SELECTOR_UNSPECIFIED = 0,

                /** CONSUMED_ANCESTOR value */
                CONSUMED_ANCESTOR = 1,

                /** SAME_INVOCATION value */
                SAME_INVOCATION = 2,

                /** PREVIOUS value */
                PREVIOUS = 3,

                /** LATEST_KNOWN_AT_ANCHOR value */
                LATEST_KNOWN_AT_ANCHOR = 4
            }

            /**
             * Properties of a Probe.
             * @deprecated Use doramcap.provenance.v1.Probe.$Properties instead.
             */
            interface IProbe extends doramcap.provenance.v1.Probe.$Properties {
            }

            /** Represents a Probe. */
            class Probe {

                /**
                 * Constructs a new Probe.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance.v1.Probe.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Probe step. */
                step?: (doramcap.provenance.v1.Step.$Properties|null);

                /** Probe boundary. */
                boundary: doramcap.provenance.v1.Boundary;

                /** Probe selector. */
                selector: doramcap.provenance.v1.Selector;

                /**
                 * Encodes the specified Probe message. Does not implicitly {@link doramcap.provenance.v1.Probe.verify|verify} messages.
                 * @param message Probe message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance.v1.Probe.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Probe message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.Probe & doramcap.provenance.v1.Probe.$Shape} Probe
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance.v1.Probe & doramcap.provenance.v1.Probe.$Shape;

                /**
                 * Creates a Probe message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Probe
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance.v1.Probe;

                /**
                 * Creates a plain object from a Probe message. Also converts values to other types if specified.
                 * @param message Probe
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance.v1.Probe, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Probe to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Probe
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Probe {

                /** Properties of a Probe. */
                interface $Properties {

                    /** Probe step */
                    step?: (doramcap.provenance.v1.Step.$Properties|null);

                    /** Probe boundary */
                    boundary?: (doramcap.provenance.v1.Boundary|null);

                    /** Probe selector */
                    selector?: (doramcap.provenance.v1.Selector|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Probe. */
                type $Shape = doramcap.provenance.v1.Probe.$Properties;
            }

            /** MissingDataPolicy enum. */
            enum MissingDataPolicy {

                /** MISSING_DATA_POLICY_UNSPECIFIED value */
                MISSING_DATA_POLICY_UNSPECIFIED = 0,

                /** WAIT_UNTIL_EVICTION value */
                WAIT_UNTIL_EVICTION = 1
            }

            /**
             * Properties of a MeasurementDefinition.
             * @deprecated Use doramcap.provenance.v1.MeasurementDefinition.$Properties instead.
             */
            interface IMeasurementDefinition extends doramcap.provenance.v1.MeasurementDefinition.$Properties {
            }

            /** Represents a MeasurementDefinition. */
            class MeasurementDefinition {

                /**
                 * Constructs a new MeasurementDefinition.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance.v1.MeasurementDefinition.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** MeasurementDefinition name. */
                name: string;

                /** MeasurementDefinition anchor. */
                anchor?: (doramcap.provenance.v1.Step.$Properties|null);

                /** MeasurementDefinition start. */
                start?: (doramcap.provenance.v1.Probe.$Properties|null);

                /** MeasurementDefinition end. */
                end?: (doramcap.provenance.v1.Probe.$Properties|null);

                /** MeasurementDefinition missingData. */
                missingData: doramcap.provenance.v1.MissingDataPolicy;

                /**
                 * Encodes the specified MeasurementDefinition message. Does not implicitly {@link doramcap.provenance.v1.MeasurementDefinition.verify|verify} messages.
                 * @param message MeasurementDefinition message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance.v1.MeasurementDefinition.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a MeasurementDefinition message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.MeasurementDefinition & doramcap.provenance.v1.MeasurementDefinition.$Shape} MeasurementDefinition
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance.v1.MeasurementDefinition & doramcap.provenance.v1.MeasurementDefinition.$Shape;

                /**
                 * Creates a MeasurementDefinition message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns MeasurementDefinition
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance.v1.MeasurementDefinition;

                /**
                 * Creates a plain object from a MeasurementDefinition message. Also converts values to other types if specified.
                 * @param message MeasurementDefinition
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance.v1.MeasurementDefinition, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this MeasurementDefinition to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for MeasurementDefinition
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace MeasurementDefinition {

                /** Properties of a MeasurementDefinition. */
                interface $Properties {

                    /** MeasurementDefinition name */
                    name?: (string|null);

                    /** MeasurementDefinition anchor */
                    anchor?: (doramcap.provenance.v1.Step.$Properties|null);

                    /** MeasurementDefinition start */
                    start?: (doramcap.provenance.v1.Probe.$Properties|null);

                    /** MeasurementDefinition end */
                    end?: (doramcap.provenance.v1.Probe.$Properties|null);

                    /** MeasurementDefinition missingData */
                    missingData?: (doramcap.provenance.v1.MissingDataPolicy|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a MeasurementDefinition. */
                type $Shape = doramcap.provenance.v1.MeasurementDefinition.$Properties;
            }

            /**
             * Properties of a Definitions.
             * @deprecated Use doramcap.provenance.v1.Definitions.$Properties instead.
             */
            interface IDefinitions extends doramcap.provenance.v1.Definitions.$Properties {
            }

            /** Represents a Definitions. */
            class Definitions {

                /**
                 * Constructs a new Definitions.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance.v1.Definitions.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Definitions version. */
                version: number;

                /** Definitions measurements. */
                measurements: doramcap.provenance.v1.MeasurementDefinition.$Properties[];

                /**
                 * Encodes the specified Definitions message. Does not implicitly {@link doramcap.provenance.v1.Definitions.verify|verify} messages.
                 * @param message Definitions message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance.v1.Definitions.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Definitions message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.Definitions & doramcap.provenance.v1.Definitions.$Shape} Definitions
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance.v1.Definitions & doramcap.provenance.v1.Definitions.$Shape;

                /**
                 * Creates a Definitions message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Definitions
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance.v1.Definitions;

                /**
                 * Creates a plain object from a Definitions message. Also converts values to other types if specified.
                 * @param message Definitions
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance.v1.Definitions, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Definitions to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Definitions
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Definitions {

                /** Properties of a Definitions. */
                interface $Properties {

                    /** Definitions version */
                    version?: (number|null);

                    /** Definitions measurements */
                    measurements?: (doramcap.provenance.v1.MeasurementDefinition.$Properties[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Definitions. */
                type $Shape = doramcap.provenance.v1.Definitions.$Properties;
            }

            /** Status enum. */
            enum Status {

                /** STATUS_UNSPECIFIED value */
                STATUS_UNSPECIFIED = 0,

                /** COMPLETE value */
                COMPLETE = 1,

                /** INCOMPLETE value */
                INCOMPLETE = 2
            }

            /** Reason enum. */
            enum Reason {

                /** REASON_UNSPECIFIED value */
                REASON_UNSPECIFIED = 0,

                /** HISTORY_EVICTED value */
                HISTORY_EVICTED = 1,

                /** ANCHOR_EVICTED value */
                ANCHOR_EVICTED = 2,

                /** NO_MATCH value */
                NO_MATCH = 3,

                /** AMBIGUOUS value */
                AMBIGUOUS = 4,

                /** CLOCK_MISMATCH value */
                CLOCK_MISMATCH = 5,

                /** INVALID_DURATION value */
                INVALID_DURATION = 6,

                /** TRAVERSAL_LIMIT value */
                TRAVERSAL_LIMIT = 7
            }

            /**
             * Properties of a SelectedProbe.
             * @deprecated Use doramcap.provenance.v1.SelectedProbe.$Properties instead.
             */
            interface ISelectedProbe extends doramcap.provenance.v1.SelectedProbe.$Properties {
            }

            /** Represents a SelectedProbe. */
            class SelectedProbe {

                /**
                 * Constructs a new SelectedProbe.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance.v1.SelectedProbe.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** SelectedProbe generation. */
                generation?: (doramcap.provenance.v1.GenerationRef.$Properties|null);

                /** SelectedProbe boundary. */
                boundary: doramcap.provenance.v1.Boundary;

                /** SelectedProbe timestampNs. */
                timestampNs?: (number|Long|null);

                /** SelectedProbe clockDomain. */
                clockDomain: string;

                /**
                 * Encodes the specified SelectedProbe message. Does not implicitly {@link doramcap.provenance.v1.SelectedProbe.verify|verify} messages.
                 * @param message SelectedProbe message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance.v1.SelectedProbe.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a SelectedProbe message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.SelectedProbe & doramcap.provenance.v1.SelectedProbe.$Shape} SelectedProbe
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance.v1.SelectedProbe & doramcap.provenance.v1.SelectedProbe.$Shape;

                /**
                 * Creates a SelectedProbe message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns SelectedProbe
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance.v1.SelectedProbe;

                /**
                 * Creates a plain object from a SelectedProbe message. Also converts values to other types if specified.
                 * @param message SelectedProbe
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance.v1.SelectedProbe, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this SelectedProbe to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for SelectedProbe
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace SelectedProbe {

                /** Properties of a SelectedProbe. */
                interface $Properties {

                    /** SelectedProbe generation */
                    generation?: (doramcap.provenance.v1.GenerationRef.$Properties|null);

                    /** SelectedProbe boundary */
                    boundary?: (doramcap.provenance.v1.Boundary|null);

                    /** SelectedProbe timestampNs */
                    timestampNs?: (number|Long|null);

                    /** SelectedProbe clockDomain */
                    clockDomain?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a SelectedProbe. */
                type $Shape = doramcap.provenance.v1.SelectedProbe.$Properties;
            }

            /**
             * Properties of a MeasurementResult.
             * @deprecated Use doramcap.provenance.v1.MeasurementResult.$Properties instead.
             */
            interface IMeasurementResult extends doramcap.provenance.v1.MeasurementResult.$Properties {
            }

            /** Represents a MeasurementResult. */
            class MeasurementResult {

                /**
                 * Constructs a new MeasurementResult.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.provenance.v1.MeasurementResult.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** MeasurementResult version. */
                version: number;

                /** MeasurementResult measurement. */
                measurement: string;

                /** MeasurementResult anchor. */
                anchor?: (doramcap.provenance.v1.GenerationRef.$Properties|null);

                /** MeasurementResult start. */
                start?: (doramcap.provenance.v1.SelectedProbe.$Properties|null);

                /** MeasurementResult end. */
                end?: (doramcap.provenance.v1.SelectedProbe.$Properties|null);

                /** MeasurementResult status. */
                status: doramcap.provenance.v1.Status;

                /** MeasurementResult reason. */
                reason: doramcap.provenance.v1.Reason;

                /** MeasurementResult durationNs. */
                durationNs?: (number|Long|null);

                /**
                 * Encodes the specified MeasurementResult message. Does not implicitly {@link doramcap.provenance.v1.MeasurementResult.verify|verify} messages.
                 * @param message MeasurementResult message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.provenance.v1.MeasurementResult.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a MeasurementResult message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.provenance.v1.MeasurementResult & doramcap.provenance.v1.MeasurementResult.$Shape} MeasurementResult
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.provenance.v1.MeasurementResult & doramcap.provenance.v1.MeasurementResult.$Shape;

                /**
                 * Creates a MeasurementResult message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns MeasurementResult
                 */
                static fromObject(object: { [k: string]: any }): doramcap.provenance.v1.MeasurementResult;

                /**
                 * Creates a plain object from a MeasurementResult message. Also converts values to other types if specified.
                 * @param message MeasurementResult
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.provenance.v1.MeasurementResult, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this MeasurementResult to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for MeasurementResult
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace MeasurementResult {

                /** Properties of a MeasurementResult. */
                interface $Properties {

                    /** MeasurementResult version */
                    version?: (number|null);

                    /** MeasurementResult measurement */
                    measurement?: (string|null);

                    /** MeasurementResult anchor */
                    anchor?: (doramcap.provenance.v1.GenerationRef.$Properties|null);

                    /** MeasurementResult start */
                    start?: (doramcap.provenance.v1.SelectedProbe.$Properties|null);

                    /** MeasurementResult end */
                    end?: (doramcap.provenance.v1.SelectedProbe.$Properties|null);

                    /** MeasurementResult status */
                    status?: (doramcap.provenance.v1.Status|null);

                    /** MeasurementResult reason */
                    reason?: (doramcap.provenance.v1.Reason|null);

                    /** MeasurementResult durationNs */
                    durationNs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a MeasurementResult. */
                type $Shape = doramcap.provenance.v1.MeasurementResult.$Properties;
            }
        }
    }

    /** Namespace activity. */
    namespace activity {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of a Batch.
             * @deprecated Use doramcap.activity.v1.Batch.$Properties instead.
             */
            interface IBatch extends doramcap.activity.v1.Batch.$Properties {
            }

            /** Represents a Batch. */
            class Batch {

                /**
                 * Constructs a new Batch.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.activity.v1.Batch.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Batch version. */
                version: number;

                /** Batch session. */
                session: (number|Long);

                /** Batch sequence. */
                sequence: (number|Long);

                /** Batch unixOriginNs. */
                unixOriginNs: (number|Long);

                /** Batch droppedRecords. */
                droppedRecords: (number|Long);

                /** Batch pid. */
                pid: number;

                /** Batch dictionary. */
                dictionary: string[];

                /** Batch strings. */
                strings: Uint8Array;

                /** Batch records. */
                records: doramcap.activity.v1.Record.$Properties[];

                /** Batch droppedFields. */
                droppedFields: (number|Long);

                /** Batch rejectedSpans. */
                rejectedSpans: (number|Long);

                /** Batch unregisteredRecords. */
                unregisteredRecords: (number|Long);

                /**
                 * Encodes the specified Batch message. Does not implicitly {@link doramcap.activity.v1.Batch.verify|verify} messages.
                 * @param message Batch message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.activity.v1.Batch.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Batch message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.activity.v1.Batch & doramcap.activity.v1.Batch.$Shape} Batch
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.activity.v1.Batch & doramcap.activity.v1.Batch.$Shape;

                /**
                 * Creates a Batch message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Batch
                 */
                static fromObject(object: { [k: string]: any }): doramcap.activity.v1.Batch;

                /**
                 * Creates a plain object from a Batch message. Also converts values to other types if specified.
                 * @param message Batch
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.activity.v1.Batch, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Batch to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Batch
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Batch {

                /** Properties of a Batch. */
                interface $Properties {

                    /** Batch version */
                    version?: (number|null);

                    /** Batch session */
                    session?: (number|Long|null);

                    /** Batch sequence */
                    sequence?: (number|Long|null);

                    /** Batch unixOriginNs */
                    unixOriginNs?: (number|Long|null);

                    /** Batch droppedRecords */
                    droppedRecords?: (number|Long|null);

                    /** Batch pid */
                    pid?: (number|null);

                    /** Batch dictionary */
                    dictionary?: (string[]|null);

                    /** Batch strings */
                    strings?: (Uint8Array|null);

                    /** Batch records */
                    records?: (doramcap.activity.v1.Record.$Properties[]|null);

                    /** Batch droppedFields */
                    droppedFields?: (number|Long|null);

                    /** Batch rejectedSpans */
                    rejectedSpans?: (number|Long|null);

                    /** Batch unregisteredRecords */
                    unregisteredRecords?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Batch. */
                type $Shape = doramcap.activity.v1.Batch.$Properties;
            }

            /**
             * Properties of a Record.
             * @deprecated Use doramcap.activity.v1.Record.$Properties instead.
             */
            interface IRecord extends doramcap.activity.v1.Record.$Properties {
            }

            /** Represents a Record. */
            class Record {

                /**
                 * Constructs a new Record.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.activity.v1.Record.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Record kind. */
                kind: doramcap.activity.v1.Record.Kind;

                /** Record monotonicNs. */
                monotonicNs: (number|Long);

                /** Record spanId. */
                spanId: (number|Long);

                /** Record traceRoot. */
                traceRoot: (number|Long);

                /** Record parentId. */
                parentId: (number|Long);

                /** Record linkedId. */
                linkedId: (number|Long);

                /** Record linkedRoot. */
                linkedRoot: (number|Long);

                /** Record tid. */
                tid: number;

                /** Record threadName. */
                threadName: number;

                /** Record name. */
                name: number;

                /** Record fields. */
                fields: doramcap.activity.v1.Field.$Properties[];

                /**
                 * Encodes the specified Record message. Does not implicitly {@link doramcap.activity.v1.Record.verify|verify} messages.
                 * @param message Record message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.activity.v1.Record.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Record message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.activity.v1.Record & doramcap.activity.v1.Record.$Shape} Record
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.activity.v1.Record & doramcap.activity.v1.Record.$Shape;

                /**
                 * Creates a Record message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Record
                 */
                static fromObject(object: { [k: string]: any }): doramcap.activity.v1.Record;

                /**
                 * Creates a plain object from a Record message. Also converts values to other types if specified.
                 * @param message Record
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.activity.v1.Record, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Record to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Record
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Record {

                /** Properties of a Record. */
                interface $Properties {

                    /** Record kind */
                    kind?: (doramcap.activity.v1.Record.Kind|null);

                    /** Record monotonicNs */
                    monotonicNs?: (number|Long|null);

                    /** Record spanId */
                    spanId?: (number|Long|null);

                    /** Record traceRoot */
                    traceRoot?: (number|Long|null);

                    /** Record parentId */
                    parentId?: (number|Long|null);

                    /** Record linkedId */
                    linkedId?: (number|Long|null);

                    /** Record linkedRoot */
                    linkedRoot?: (number|Long|null);

                    /** Record tid */
                    tid?: (number|null);

                    /** Record threadName */
                    threadName?: (number|null);

                    /** Record name */
                    name?: (number|null);

                    /** Record fields */
                    fields?: (doramcap.activity.v1.Field.$Properties[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Record. */
                type $Shape = doramcap.activity.v1.Record.$Properties;

                /** Kind enum. */
                enum Kind {

                    /** BEGIN value */
                    BEGIN = 0,

                    /** EVENT value */
                    EVENT = 1,

                    /** UPDATE value */
                    UPDATE = 2,

                    /** LINK value */
                    LINK = 3,

                    /** END value */
                    END = 4
                }
            }

            /**
             * Properties of a Field.
             * @deprecated Use doramcap.activity.v1.Field.$Properties instead.
             */
            interface IField extends doramcap.activity.v1.Field.$Properties {
            }

            /** Represents a Field. */
            class Field {

                /**
                 * Constructs a new Field.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.activity.v1.Field.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Field name. */
                name: number;

                /** Field kind. */
                kind: doramcap.activity.v1.Field.Kind;

                /** Field bits. */
                bits: (number|Long);

                /** Field offset. */
                offset: number;

                /** Field length. */
                length: number;

                /**
                 * Encodes the specified Field message. Does not implicitly {@link doramcap.activity.v1.Field.verify|verify} messages.
                 * @param message Field message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.activity.v1.Field.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Field message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.activity.v1.Field & doramcap.activity.v1.Field.$Shape} Field
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.activity.v1.Field & doramcap.activity.v1.Field.$Shape;

                /**
                 * Creates a Field message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Field
                 */
                static fromObject(object: { [k: string]: any }): doramcap.activity.v1.Field;

                /**
                 * Creates a plain object from a Field message. Also converts values to other types if specified.
                 * @param message Field
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.activity.v1.Field, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Field to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Field
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Field {

                /** Properties of a Field. */
                interface $Properties {

                    /** Field name */
                    name?: (number|null);

                    /** Field kind */
                    kind?: (doramcap.activity.v1.Field.Kind|null);

                    /** Field bits */
                    bits?: (number|Long|null);

                    /** Field offset */
                    offset?: (number|null);

                    /** Field length */
                    length?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Field. */
                type $Shape = doramcap.activity.v1.Field.$Properties;

                /** Kind enum. */
                enum Kind {

                    /** UNSIGNED value */
                    UNSIGNED = 0,

                    /** SIGNED value */
                    SIGNED = 1,

                    /** FLOAT value */
                    FLOAT = 2,

                    /** BOOL value */
                    BOOL = 3,

                    /** STRING value */
                    STRING = 4
                }
            }
        }
    }
}
