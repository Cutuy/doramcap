import * as $protobuf from "protobufjs";
import Long = require("long");

/** Namespace doramcap. */
export namespace doramcap {

    /** Namespace console. */
    namespace console {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of a SystemAppSelection.
             * @deprecated Use doramcap.console.v1.SystemAppSelection.$Properties instead.
             */
            interface ISystemAppSelection extends doramcap.console.v1.SystemAppSelection.$Properties {
            }

            /** Represents a SystemAppSelection. */
            class SystemAppSelection {

                /**
                 * Constructs a new SystemAppSelection.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.SystemAppSelection.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** SystemAppSelection ids. */
                ids: string[];

                /** SystemAppSelection profiles. */
                profiles: { [k: string]: string };

                /**
                 * Creates a new SystemAppSelection instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns SystemAppSelection instance
                 */
                static create(properties: doramcap.console.v1.SystemAppSelection.$Shape): doramcap.console.v1.SystemAppSelection & doramcap.console.v1.SystemAppSelection.$Shape;
                static create(properties?: doramcap.console.v1.SystemAppSelection.$Properties): doramcap.console.v1.SystemAppSelection;

                /**
                 * Encodes the specified SystemAppSelection message. Does not implicitly {@link doramcap.console.v1.SystemAppSelection.verify|verify} messages.
                 * @param message SystemAppSelection message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.SystemAppSelection, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified SystemAppSelection message, length delimited. Does not implicitly {@link doramcap.console.v1.SystemAppSelection.verify|verify} messages.
                 * @param message SystemAppSelection message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.SystemAppSelection, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a SystemAppSelection message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.SystemAppSelection & doramcap.console.v1.SystemAppSelection.$Shape} SystemAppSelection
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.SystemAppSelection & doramcap.console.v1.SystemAppSelection.$Shape;

                /**
                 * Decodes a SystemAppSelection message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.SystemAppSelection & doramcap.console.v1.SystemAppSelection.$Shape} SystemAppSelection
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.SystemAppSelection & doramcap.console.v1.SystemAppSelection.$Shape;

                /**
                 * Creates a SystemAppSelection message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns SystemAppSelection
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.SystemAppSelection;

                /**
                 * Creates a plain object from a SystemAppSelection message. Also converts values to other types if specified.
                 * @param message SystemAppSelection
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.SystemAppSelection, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this SystemAppSelection to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for SystemAppSelection
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace SystemAppSelection {

                /** Properties of a SystemAppSelection. */
                interface $Properties {

                    /** SystemAppSelection ids */
                    ids?: (string[]|null);

                    /** SystemAppSelection profiles */
                    profiles?: ({ [k: string]: string }|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a SystemAppSelection. */
                type $Shape = doramcap.console.v1.SystemAppSelection.$Properties;
            }

            /**
             * Properties of a ProfileOption.
             * @deprecated Use doramcap.console.v1.ProfileOption.$Properties instead.
             */
            interface IProfileOption extends doramcap.console.v1.ProfileOption.$Properties {
            }

            /** Represents a ProfileOption. */
            class ProfileOption {

                /**
                 * Constructs a new ProfileOption.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.ProfileOption.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ProfileOption id. */
                id: string;

                /** ProfileOption label. */
                label: string;

                /**
                 * Creates a new ProfileOption instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ProfileOption instance
                 */
                static create(properties: doramcap.console.v1.ProfileOption.$Shape): doramcap.console.v1.ProfileOption & doramcap.console.v1.ProfileOption.$Shape;
                static create(properties?: doramcap.console.v1.ProfileOption.$Properties): doramcap.console.v1.ProfileOption;

                /**
                 * Encodes the specified ProfileOption message. Does not implicitly {@link doramcap.console.v1.ProfileOption.verify|verify} messages.
                 * @param message ProfileOption message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.ProfileOption, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ProfileOption message, length delimited. Does not implicitly {@link doramcap.console.v1.ProfileOption.verify|verify} messages.
                 * @param message ProfileOption message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.ProfileOption, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ProfileOption message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.ProfileOption & doramcap.console.v1.ProfileOption.$Shape} ProfileOption
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.ProfileOption & doramcap.console.v1.ProfileOption.$Shape;

                /**
                 * Decodes a ProfileOption message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.ProfileOption & doramcap.console.v1.ProfileOption.$Shape} ProfileOption
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.ProfileOption & doramcap.console.v1.ProfileOption.$Shape;

                /**
                 * Creates a ProfileOption message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ProfileOption
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.ProfileOption;

                /**
                 * Creates a plain object from a ProfileOption message. Also converts values to other types if specified.
                 * @param message ProfileOption
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.ProfileOption, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ProfileOption to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ProfileOption
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ProfileOption {

                /** Properties of a ProfileOption. */
                interface $Properties {

                    /** ProfileOption id */
                    id?: (string|null);

                    /** ProfileOption label */
                    label?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a ProfileOption. */
                type $Shape = doramcap.console.v1.ProfileOption.$Properties;
            }

            /**
             * Properties of a SystemApp.
             * @deprecated Use doramcap.console.v1.SystemApp.$Properties instead.
             */
            interface ISystemApp extends doramcap.console.v1.SystemApp.$Properties {
            }

            /** Represents a SystemApp. */
            class SystemApp {

                /**
                 * Constructs a new SystemApp.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.SystemApp.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** SystemApp id. */
                id: string;

                /** SystemApp label. */
                label: string;

                /** SystemApp selected. */
                selected: boolean;

                /** SystemApp required. */
                required: boolean;

                /** SystemApp profiles. */
                profiles: doramcap.console.v1.ProfileOption[];

                /** SystemApp profile. */
                profile: string;

                /**
                 * Creates a new SystemApp instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns SystemApp instance
                 */
                static create(properties: doramcap.console.v1.SystemApp.$Shape): doramcap.console.v1.SystemApp & doramcap.console.v1.SystemApp.$Shape;
                static create(properties?: doramcap.console.v1.SystemApp.$Properties): doramcap.console.v1.SystemApp;

                /**
                 * Encodes the specified SystemApp message. Does not implicitly {@link doramcap.console.v1.SystemApp.verify|verify} messages.
                 * @param message SystemApp message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.SystemApp, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified SystemApp message, length delimited. Does not implicitly {@link doramcap.console.v1.SystemApp.verify|verify} messages.
                 * @param message SystemApp message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.SystemApp, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a SystemApp message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.SystemApp & doramcap.console.v1.SystemApp.$Shape} SystemApp
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.SystemApp & doramcap.console.v1.SystemApp.$Shape;

                /**
                 * Decodes a SystemApp message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.SystemApp & doramcap.console.v1.SystemApp.$Shape} SystemApp
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.SystemApp & doramcap.console.v1.SystemApp.$Shape;

                /**
                 * Creates a SystemApp message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns SystemApp
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.SystemApp;

                /**
                 * Creates a plain object from a SystemApp message. Also converts values to other types if specified.
                 * @param message SystemApp
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.SystemApp, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this SystemApp to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for SystemApp
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace SystemApp {

                /** Properties of a SystemApp. */
                interface $Properties {

                    /** SystemApp id */
                    id?: (string|null);

                    /** SystemApp label */
                    label?: (string|null);

                    /** SystemApp selected */
                    selected?: (boolean|null);

                    /** SystemApp required */
                    required?: (boolean|null);

                    /** SystemApp profiles */
                    profiles?: (doramcap.console.v1.ProfileOption[]|null);

                    /** SystemApp profile */
                    profile?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a SystemApp. */
                type $Shape = doramcap.console.v1.SystemApp.$Properties;
            }

            /**
             * Properties of a PrepareRequest.
             * @deprecated Use doramcap.console.v1.PrepareRequest.$Properties instead.
             */
            interface IPrepareRequest extends doramcap.console.v1.PrepareRequest.$Properties {
            }

            /** Represents a PrepareRequest. */
            class PrepareRequest {

                /**
                 * Constructs a new PrepareRequest.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.PrepareRequest.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** PrepareRequest profiles. */
                profiles: string[];

                /** PrepareRequest systemApps. */
                systemApps?: (doramcap.console.v1.SystemAppSelection|null);

                /**
                 * Creates a new PrepareRequest instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns PrepareRequest instance
                 */
                static create(properties: doramcap.console.v1.PrepareRequest.$Shape): doramcap.console.v1.PrepareRequest & doramcap.console.v1.PrepareRequest.$Shape;
                static create(properties?: doramcap.console.v1.PrepareRequest.$Properties): doramcap.console.v1.PrepareRequest;

                /**
                 * Encodes the specified PrepareRequest message. Does not implicitly {@link doramcap.console.v1.PrepareRequest.verify|verify} messages.
                 * @param message PrepareRequest message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.PrepareRequest, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified PrepareRequest message, length delimited. Does not implicitly {@link doramcap.console.v1.PrepareRequest.verify|verify} messages.
                 * @param message PrepareRequest message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.PrepareRequest, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a PrepareRequest message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.PrepareRequest & doramcap.console.v1.PrepareRequest.$Shape} PrepareRequest
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.PrepareRequest & doramcap.console.v1.PrepareRequest.$Shape;

                /**
                 * Decodes a PrepareRequest message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.PrepareRequest & doramcap.console.v1.PrepareRequest.$Shape} PrepareRequest
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.PrepareRequest & doramcap.console.v1.PrepareRequest.$Shape;

                /**
                 * Creates a PrepareRequest message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns PrepareRequest
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.PrepareRequest;

                /**
                 * Creates a plain object from a PrepareRequest message. Also converts values to other types if specified.
                 * @param message PrepareRequest
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.PrepareRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this PrepareRequest to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for PrepareRequest
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace PrepareRequest {

                /** Properties of a PrepareRequest. */
                interface $Properties {

                    /** PrepareRequest profiles */
                    profiles?: (string[]|null);

                    /** PrepareRequest systemApps */
                    systemApps?: (doramcap.console.v1.SystemAppSelection|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a PrepareRequest. */
                type $Shape = doramcap.console.v1.PrepareRequest.$Properties;
            }

            /**
             * Properties of a StartRequest.
             * @deprecated Use doramcap.console.v1.StartRequest.$Properties instead.
             */
            interface IStartRequest extends doramcap.console.v1.StartRequest.$Properties {
            }

            /** Represents a StartRequest. */
            class StartRequest {

                /**
                 * Constructs a new StartRequest.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.StartRequest.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** StartRequest preparedId. */
                preparedId: string;

                /** StartRequest requestKey. */
                requestKey: string;

                /**
                 * Creates a new StartRequest instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns StartRequest instance
                 */
                static create(properties: doramcap.console.v1.StartRequest.$Shape): doramcap.console.v1.StartRequest & doramcap.console.v1.StartRequest.$Shape;
                static create(properties?: doramcap.console.v1.StartRequest.$Properties): doramcap.console.v1.StartRequest;

                /**
                 * Encodes the specified StartRequest message. Does not implicitly {@link doramcap.console.v1.StartRequest.verify|verify} messages.
                 * @param message StartRequest message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.StartRequest, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified StartRequest message, length delimited. Does not implicitly {@link doramcap.console.v1.StartRequest.verify|verify} messages.
                 * @param message StartRequest message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.StartRequest, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a StartRequest message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.StartRequest & doramcap.console.v1.StartRequest.$Shape} StartRequest
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.StartRequest & doramcap.console.v1.StartRequest.$Shape;

                /**
                 * Decodes a StartRequest message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.StartRequest & doramcap.console.v1.StartRequest.$Shape} StartRequest
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.StartRequest & doramcap.console.v1.StartRequest.$Shape;

                /**
                 * Creates a StartRequest message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns StartRequest
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.StartRequest;

                /**
                 * Creates a plain object from a StartRequest message. Also converts values to other types if specified.
                 * @param message StartRequest
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.StartRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this StartRequest to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for StartRequest
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace StartRequest {

                /** Properties of a StartRequest. */
                interface $Properties {

                    /** StartRequest preparedId */
                    preparedId?: (string|null);

                    /** StartRequest requestKey */
                    requestKey?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a StartRequest. */
                type $Shape = doramcap.console.v1.StartRequest.$Properties;
            }

            /**
             * Properties of a StopRequest.
             * @deprecated Use doramcap.console.v1.StopRequest.$Properties instead.
             */
            interface IStopRequest extends doramcap.console.v1.StopRequest.$Properties {
            }

            /** Represents a StopRequest. */
            class StopRequest {

                /**
                 * Constructs a new StopRequest.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.StopRequest.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** StopRequest runId. */
                runId: string;

                /**
                 * Creates a new StopRequest instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns StopRequest instance
                 */
                static create(properties: doramcap.console.v1.StopRequest.$Shape): doramcap.console.v1.StopRequest & doramcap.console.v1.StopRequest.$Shape;
                static create(properties?: doramcap.console.v1.StopRequest.$Properties): doramcap.console.v1.StopRequest;

                /**
                 * Encodes the specified StopRequest message. Does not implicitly {@link doramcap.console.v1.StopRequest.verify|verify} messages.
                 * @param message StopRequest message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.StopRequest, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified StopRequest message, length delimited. Does not implicitly {@link doramcap.console.v1.StopRequest.verify|verify} messages.
                 * @param message StopRequest message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.StopRequest, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a StopRequest message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.StopRequest & doramcap.console.v1.StopRequest.$Shape} StopRequest
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.StopRequest & doramcap.console.v1.StopRequest.$Shape;

                /**
                 * Decodes a StopRequest message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.StopRequest & doramcap.console.v1.StopRequest.$Shape} StopRequest
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.StopRequest & doramcap.console.v1.StopRequest.$Shape;

                /**
                 * Creates a StopRequest message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns StopRequest
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.StopRequest;

                /**
                 * Creates a plain object from a StopRequest message. Also converts values to other types if specified.
                 * @param message StopRequest
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.StopRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this StopRequest to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for StopRequest
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace StopRequest {

                /** Properties of a StopRequest. */
                interface $Properties {

                    /** StopRequest runId */
                    runId?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a StopRequest. */
                type $Shape = doramcap.console.v1.StopRequest.$Properties;
            }

            /**
             * Properties of a StartResponse.
             * @deprecated Use doramcap.console.v1.StartResponse.$Properties instead.
             */
            interface IStartResponse extends doramcap.console.v1.StartResponse.$Properties {
            }

            /** Represents a StartResponse. */
            class StartResponse {

                /**
                 * Constructs a new StartResponse.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.StartResponse.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** StartResponse runId. */
                runId: string;

                /**
                 * Creates a new StartResponse instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns StartResponse instance
                 */
                static create(properties: doramcap.console.v1.StartResponse.$Shape): doramcap.console.v1.StartResponse & doramcap.console.v1.StartResponse.$Shape;
                static create(properties?: doramcap.console.v1.StartResponse.$Properties): doramcap.console.v1.StartResponse;

                /**
                 * Encodes the specified StartResponse message. Does not implicitly {@link doramcap.console.v1.StartResponse.verify|verify} messages.
                 * @param message StartResponse message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.StartResponse, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified StartResponse message, length delimited. Does not implicitly {@link doramcap.console.v1.StartResponse.verify|verify} messages.
                 * @param message StartResponse message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.StartResponse, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a StartResponse message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.StartResponse & doramcap.console.v1.StartResponse.$Shape} StartResponse
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.StartResponse & doramcap.console.v1.StartResponse.$Shape;

                /**
                 * Decodes a StartResponse message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.StartResponse & doramcap.console.v1.StartResponse.$Shape} StartResponse
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.StartResponse & doramcap.console.v1.StartResponse.$Shape;

                /**
                 * Creates a StartResponse message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns StartResponse
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.StartResponse;

                /**
                 * Creates a plain object from a StartResponse message. Also converts values to other types if specified.
                 * @param message StartResponse
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.StartResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this StartResponse to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for StartResponse
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace StartResponse {

                /** Properties of a StartResponse. */
                interface $Properties {

                    /** StartResponse runId */
                    runId?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a StartResponse. */
                type $Shape = doramcap.console.v1.StartResponse.$Properties;
            }

            /**
             * Properties of a StopResponse.
             * @deprecated Use doramcap.console.v1.StopResponse.$Properties instead.
             */
            interface IStopResponse extends doramcap.console.v1.StopResponse.$Properties {
            }

            /** Represents a StopResponse. */
            class StopResponse {

                /**
                 * Constructs a new StopResponse.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.StopResponse.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** StopResponse accepted. */
                accepted: boolean;

                /**
                 * Creates a new StopResponse instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns StopResponse instance
                 */
                static create(properties: doramcap.console.v1.StopResponse.$Shape): doramcap.console.v1.StopResponse & doramcap.console.v1.StopResponse.$Shape;
                static create(properties?: doramcap.console.v1.StopResponse.$Properties): doramcap.console.v1.StopResponse;

                /**
                 * Encodes the specified StopResponse message. Does not implicitly {@link doramcap.console.v1.StopResponse.verify|verify} messages.
                 * @param message StopResponse message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.StopResponse, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified StopResponse message, length delimited. Does not implicitly {@link doramcap.console.v1.StopResponse.verify|verify} messages.
                 * @param message StopResponse message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.StopResponse, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a StopResponse message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.StopResponse & doramcap.console.v1.StopResponse.$Shape} StopResponse
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.StopResponse & doramcap.console.v1.StopResponse.$Shape;

                /**
                 * Decodes a StopResponse message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.StopResponse & doramcap.console.v1.StopResponse.$Shape} StopResponse
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.StopResponse & doramcap.console.v1.StopResponse.$Shape;

                /**
                 * Creates a StopResponse message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns StopResponse
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.StopResponse;

                /**
                 * Creates a plain object from a StopResponse message. Also converts values to other types if specified.
                 * @param message StopResponse
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.StopResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this StopResponse to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for StopResponse
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace StopResponse {

                /** Properties of a StopResponse. */
                interface $Properties {

                    /** StopResponse accepted */
                    accepted?: (boolean|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a StopResponse. */
                type $Shape = doramcap.console.v1.StopResponse.$Properties;
            }

            /**
             * Properties of a Service.
             * @deprecated Use doramcap.console.v1.Service.$Properties instead.
             */
            interface IService extends doramcap.console.v1.Service.$Properties {
            }

            /** Represents a Service. */
            class Service {

                /**
                 * Constructs a new Service.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.Service.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Service name. */
                name: string;

                /** Service imageId. */
                imageId: string;

                /**
                 * Creates a new Service instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Service instance
                 */
                static create(properties: doramcap.console.v1.Service.$Shape): doramcap.console.v1.Service & doramcap.console.v1.Service.$Shape;
                static create(properties?: doramcap.console.v1.Service.$Properties): doramcap.console.v1.Service;

                /**
                 * Encodes the specified Service message. Does not implicitly {@link doramcap.console.v1.Service.verify|verify} messages.
                 * @param message Service message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.Service, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Service message, length delimited. Does not implicitly {@link doramcap.console.v1.Service.verify|verify} messages.
                 * @param message Service message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.Service, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Service message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.Service & doramcap.console.v1.Service.$Shape} Service
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.Service & doramcap.console.v1.Service.$Shape;

                /**
                 * Decodes a Service message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.Service & doramcap.console.v1.Service.$Shape} Service
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.Service & doramcap.console.v1.Service.$Shape;

                /**
                 * Creates a Service message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Service
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.Service;

                /**
                 * Creates a plain object from a Service message. Also converts values to other types if specified.
                 * @param message Service
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.Service, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Service to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Service
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Service {

                /** Properties of a Service. */
                interface $Properties {

                    /** Service name */
                    name?: (string|null);

                    /** Service imageId */
                    imageId?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Service. */
                type $Shape = doramcap.console.v1.Service.$Properties;
            }

            /**
             * Properties of a Review.
             * @deprecated Use doramcap.console.v1.Review.$Properties instead.
             */
            interface IReview extends doramcap.console.v1.Review.$Properties {
            }

            /** Represents a Review. */
            class Review {

                /**
                 * Constructs a new Review.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.Review.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Review preparedId. */
                preparedId: string;

                /** Review runId. */
                runId: string;

                /** Review availableProfiles. */
                availableProfiles: string[];

                /** Review selectedProfiles. */
                selectedProfiles: string[];

                /** Review services. */
                services: doramcap.console.v1.Service[];

                /** Review warnings. */
                warnings: string[];

                /** Review systemApps. */
                systemApps: doramcap.console.v1.SystemApp[];

                /** Review recordings. */
                recordings: doramcap.console.v1.RecordingOutput[];

                /** Review liveStreams. */
                liveStreams: doramcap.console.v1.LiveStream[];

                /**
                 * Creates a new Review instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Review instance
                 */
                static create(properties: doramcap.console.v1.Review.$Shape): doramcap.console.v1.Review & doramcap.console.v1.Review.$Shape;
                static create(properties?: doramcap.console.v1.Review.$Properties): doramcap.console.v1.Review;

                /**
                 * Encodes the specified Review message. Does not implicitly {@link doramcap.console.v1.Review.verify|verify} messages.
                 * @param message Review message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.Review, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Review message, length delimited. Does not implicitly {@link doramcap.console.v1.Review.verify|verify} messages.
                 * @param message Review message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.Review, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Review message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.Review & doramcap.console.v1.Review.$Shape} Review
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.Review & doramcap.console.v1.Review.$Shape;

                /**
                 * Decodes a Review message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.Review & doramcap.console.v1.Review.$Shape} Review
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.Review & doramcap.console.v1.Review.$Shape;

                /**
                 * Creates a Review message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Review
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.Review;

                /**
                 * Creates a plain object from a Review message. Also converts values to other types if specified.
                 * @param message Review
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.Review, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Review to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Review
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Review {

                /** Properties of a Review. */
                interface $Properties {

                    /** Review preparedId */
                    preparedId?: (string|null);

                    /** Review runId */
                    runId?: (string|null);

                    /** Review availableProfiles */
                    availableProfiles?: (string[]|null);

                    /** Review selectedProfiles */
                    selectedProfiles?: (string[]|null);

                    /** Review services */
                    services?: (doramcap.console.v1.Service[]|null);

                    /** Review warnings */
                    warnings?: (string[]|null);

                    /** Review systemApps */
                    systemApps?: (doramcap.console.v1.SystemApp[]|null);

                    /** Review recordings */
                    recordings?: (doramcap.console.v1.RecordingOutput[]|null);

                    /** Review liveStreams */
                    liveStreams?: (doramcap.console.v1.LiveStream[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Review. */
                type $Shape = doramcap.console.v1.Review.$Properties;
            }

            /** Phase enum. */
            enum Phase {

                /** PHASE_UNSPECIFIED value */
                PHASE_UNSPECIFIED = 0,

                /** IDLE value */
                IDLE = 1,

                /** PREPARING value */
                PREPARING = 2,

                /** REVIEWED value */
                REVIEWED = 3,

                /** STARTING value */
                STARTING = 4,

                /** RUNNING value */
                RUNNING = 5,

                /** STOPPING value */
                STOPPING = 6,

                /** FINISHED value */
                FINISHED = 7,

                /** BLOCKED value */
                BLOCKED = 8
            }

            /**
             * Properties of an Outcome.
             * @deprecated Use doramcap.console.v1.Outcome.$Properties instead.
             */
            interface IOutcome extends doramcap.console.v1.Outcome.$Properties {
            }

            /** Represents an Outcome. */
            class Outcome {

                /**
                 * Constructs a new Outcome.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.Outcome.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Outcome execution. */
                execution: string;

                /** Outcome error. */
                error: string;

                /** Outcome writersStopped. */
                writersStopped?: (boolean|null);

                /** Outcome recordingFinalized. */
                recordingFinalized?: (boolean|null);

                /** Outcome boundaryClean. */
                boundaryClean?: (boolean|null);

                /** Outcome eventStreamHealthy. */
                eventStreamHealthy?: (boolean|null);

                /**
                 * Creates a new Outcome instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Outcome instance
                 */
                static create(properties: doramcap.console.v1.Outcome.$Shape): doramcap.console.v1.Outcome & doramcap.console.v1.Outcome.$Shape;
                static create(properties?: doramcap.console.v1.Outcome.$Properties): doramcap.console.v1.Outcome;

                /**
                 * Encodes the specified Outcome message. Does not implicitly {@link doramcap.console.v1.Outcome.verify|verify} messages.
                 * @param message Outcome message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.Outcome, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Outcome message, length delimited. Does not implicitly {@link doramcap.console.v1.Outcome.verify|verify} messages.
                 * @param message Outcome message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.Outcome, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Outcome message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.Outcome & doramcap.console.v1.Outcome.$Shape} Outcome
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.Outcome & doramcap.console.v1.Outcome.$Shape;

                /**
                 * Decodes an Outcome message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.Outcome & doramcap.console.v1.Outcome.$Shape} Outcome
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.Outcome & doramcap.console.v1.Outcome.$Shape;

                /**
                 * Creates an Outcome message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Outcome
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.Outcome;

                /**
                 * Creates a plain object from an Outcome message. Also converts values to other types if specified.
                 * @param message Outcome
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.Outcome, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Outcome to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Outcome
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Outcome {

                /** Properties of an Outcome. */
                interface $Properties {

                    /** Outcome execution */
                    execution?: (string|null);

                    /** Outcome error */
                    error?: (string|null);

                    /** Outcome writersStopped */
                    writersStopped?: (boolean|null);

                    /** Outcome recordingFinalized */
                    recordingFinalized?: (boolean|null);

                    /** Outcome boundaryClean */
                    boundaryClean?: (boolean|null);

                    /** Outcome eventStreamHealthy */
                    eventStreamHealthy?: (boolean|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an Outcome. */
                type $Shape = doramcap.console.v1.Outcome.$Properties;
            }

            /**
             * Properties of a LiveStream.
             * @deprecated Use doramcap.console.v1.LiveStream.$Properties instead.
             */
            interface ILiveStream extends doramcap.console.v1.LiveStream.$Properties {
            }

            /** Represents a LiveStream. */
            class LiveStream {

                /**
                 * Constructs a new LiveStream.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.LiveStream.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** LiveStream id. */
                id: string;

                /** LiveStream appId. */
                appId: string;

                /** LiveStream kind. */
                kind: doramcap.console.live.v1.Kind;

                /**
                 * Creates a new LiveStream instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns LiveStream instance
                 */
                static create(properties: doramcap.console.v1.LiveStream.$Shape): doramcap.console.v1.LiveStream & doramcap.console.v1.LiveStream.$Shape;
                static create(properties?: doramcap.console.v1.LiveStream.$Properties): doramcap.console.v1.LiveStream;

                /**
                 * Encodes the specified LiveStream message. Does not implicitly {@link doramcap.console.v1.LiveStream.verify|verify} messages.
                 * @param message LiveStream message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.LiveStream, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified LiveStream message, length delimited. Does not implicitly {@link doramcap.console.v1.LiveStream.verify|verify} messages.
                 * @param message LiveStream message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.LiveStream, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a LiveStream message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.LiveStream & doramcap.console.v1.LiveStream.$Shape} LiveStream
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.LiveStream & doramcap.console.v1.LiveStream.$Shape;

                /**
                 * Decodes a LiveStream message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.LiveStream & doramcap.console.v1.LiveStream.$Shape} LiveStream
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.LiveStream & doramcap.console.v1.LiveStream.$Shape;

                /**
                 * Creates a LiveStream message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns LiveStream
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.LiveStream;

                /**
                 * Creates a plain object from a LiveStream message. Also converts values to other types if specified.
                 * @param message LiveStream
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.LiveStream, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this LiveStream to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for LiveStream
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace LiveStream {

                /** Properties of a LiveStream. */
                interface $Properties {

                    /** LiveStream id */
                    id?: (string|null);

                    /** LiveStream appId */
                    appId?: (string|null);

                    /** LiveStream kind */
                    kind?: (doramcap.console.live.v1.Kind|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a LiveStream. */
                type $Shape = doramcap.console.v1.LiveStream.$Properties;
            }

            /**
             * Properties of a LiveMessage.
             * @deprecated Use doramcap.console.v1.LiveMessage.$Properties instead.
             */
            interface ILiveMessage extends doramcap.console.v1.LiveMessage.$Properties {
            }

            /** Represents a LiveMessage. */
            class LiveMessage {

                /**
                 * Constructs a new LiveMessage.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.LiveMessage.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** LiveMessage status. */
                status?: (Uint8Array|null);

                /** LiveMessage frame. */
                frame?: (Uint8Array|null);

                /** LiveMessage body. */
                body?: ("status"|"frame");

                /**
                 * Creates a new LiveMessage instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns LiveMessage instance
                 */
                static create(properties: doramcap.console.v1.LiveMessage.$Shape): doramcap.console.v1.LiveMessage & doramcap.console.v1.LiveMessage.$Shape;
                static create(properties?: doramcap.console.v1.LiveMessage.$Properties): doramcap.console.v1.LiveMessage;

                /**
                 * Encodes the specified LiveMessage message. Does not implicitly {@link doramcap.console.v1.LiveMessage.verify|verify} messages.
                 * @param message LiveMessage message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.LiveMessage, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified LiveMessage message, length delimited. Does not implicitly {@link doramcap.console.v1.LiveMessage.verify|verify} messages.
                 * @param message LiveMessage message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.LiveMessage, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a LiveMessage message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.LiveMessage & doramcap.console.v1.LiveMessage.$Shape} LiveMessage
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.LiveMessage & doramcap.console.v1.LiveMessage.$Shape;

                /**
                 * Decodes a LiveMessage message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.LiveMessage & doramcap.console.v1.LiveMessage.$Shape} LiveMessage
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.LiveMessage & doramcap.console.v1.LiveMessage.$Shape;

                /**
                 * Creates a LiveMessage message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns LiveMessage
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.LiveMessage;

                /**
                 * Creates a plain object from a LiveMessage message. Also converts values to other types if specified.
                 * @param message LiveMessage
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.LiveMessage, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this LiveMessage to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for LiveMessage
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace LiveMessage {

                /** Properties of a LiveMessage. */
                interface $Properties {

                    /** LiveMessage status */
                    status?: (Uint8Array|null);

                    /** LiveMessage frame */
                    frame?: (Uint8Array|null);

                    /** LiveMessage body */
                    body?: ("status"|"frame");

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Narrowed shape of a LiveMessage. */
                type $Shape = {
                  status?: Uint8Array|null;
                  frame?: Uint8Array|null;
                  $unknowns?: Uint8Array[];
                } & (
                  ({ body?: undefined; status?: null; frame?: null }|{ body?: "status"; status: Uint8Array; frame?: null }|{ body?: "frame"; status?: null; frame: Uint8Array })
                );
            }

            /**
             * Properties of a Status.
             * @deprecated Use doramcap.console.v1.Status.$Properties instead.
             */
            interface IStatus extends doramcap.console.v1.Status.$Properties {
            }

            /** Represents a Status. */
            class Status {

                /**
                 * Constructs a new Status.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.Status.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Status phase. */
                phase: doramcap.console.v1.Phase;

                /** Status runId. */
                runId: string;

                /** Status steps. */
                steps: doramcap.workflow.v1.Event[];

                /** Status sequence. */
                sequence: (number|Long);

                /** Status lostEvents. */
                lostEvents: (number|Long);

                /** Status truncatedSteps. */
                truncatedSteps: (number|Long);

                /** Status progressIncomplete. */
                progressIncomplete: boolean;

                /** Status outcome. */
                outcome?: (doramcap.console.v1.Outcome|null);

                /** Status overflowSteps. */
                overflowSteps: (number|Long);

                /** Status cpuAvailable. */
                cpuAvailable: boolean;

                /** Status reportAvailable. */
                reportAvailable: boolean;

                /** Status projectDirectory. */
                projectDirectory: string;

                /** Status profiles. */
                profiles: string[];

                /** Status metrics. */
                metrics: doramcap.console.v1.RunMetric[];

                /** Status activityAvailable. */
                activityAvailable: boolean;

                /** Status metricsIncomplete. */
                metricsIncomplete: boolean;

                /** Status systemApps. */
                systemApps?: (doramcap.console.v1.SystemAppSelection|null);

                /** Status recordings. */
                recordings: doramcap.console.v1.RecordingOutput[];

                /** Status liveStreams. */
                liveStreams: doramcap.console.v1.LiveStream[];

                /**
                 * Creates a new Status instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Status instance
                 */
                static create(properties: doramcap.console.v1.Status.$Shape): doramcap.console.v1.Status & doramcap.console.v1.Status.$Shape;
                static create(properties?: doramcap.console.v1.Status.$Properties): doramcap.console.v1.Status;

                /**
                 * Encodes the specified Status message. Does not implicitly {@link doramcap.console.v1.Status.verify|verify} messages.
                 * @param message Status message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.Status, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Status message, length delimited. Does not implicitly {@link doramcap.console.v1.Status.verify|verify} messages.
                 * @param message Status message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.Status, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Status message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.Status & doramcap.console.v1.Status.$Shape} Status
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.Status & doramcap.console.v1.Status.$Shape;

                /**
                 * Decodes a Status message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.Status & doramcap.console.v1.Status.$Shape} Status
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.Status & doramcap.console.v1.Status.$Shape;

                /**
                 * Creates a Status message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Status
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.Status;

                /**
                 * Creates a plain object from a Status message. Also converts values to other types if specified.
                 * @param message Status
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.Status, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Status to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Status
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Status {

                /** Properties of a Status. */
                interface $Properties {

                    /** Status phase */
                    phase?: (doramcap.console.v1.Phase|null);

                    /** Status runId */
                    runId?: (string|null);

                    /** Status steps */
                    steps?: (doramcap.workflow.v1.Event[]|null);

                    /** Status sequence */
                    sequence?: (number|Long|null);

                    /** Status lostEvents */
                    lostEvents?: (number|Long|null);

                    /** Status truncatedSteps */
                    truncatedSteps?: (number|Long|null);

                    /** Status progressIncomplete */
                    progressIncomplete?: (boolean|null);

                    /** Status outcome */
                    outcome?: (doramcap.console.v1.Outcome|null);

                    /** Status overflowSteps */
                    overflowSteps?: (number|Long|null);

                    /** Status cpuAvailable */
                    cpuAvailable?: (boolean|null);

                    /** Status reportAvailable */
                    reportAvailable?: (boolean|null);

                    /** Status projectDirectory */
                    projectDirectory?: (string|null);

                    /** Status profiles */
                    profiles?: (string[]|null);

                    /** Status metrics */
                    metrics?: (doramcap.console.v1.RunMetric[]|null);

                    /** Status activityAvailable */
                    activityAvailable?: (boolean|null);

                    /** Status metricsIncomplete */
                    metricsIncomplete?: (boolean|null);

                    /** Status systemApps */
                    systemApps?: (doramcap.console.v1.SystemAppSelection|null);

                    /** Status recordings */
                    recordings?: (doramcap.console.v1.RecordingOutput[]|null);

                    /** Status liveStreams */
                    liveStreams?: (doramcap.console.v1.LiveStream[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Status. */
                type $Shape = doramcap.console.v1.Status.$Properties;
            }

            /**
             * Properties of a ConsoleError.
             * @deprecated Use doramcap.console.v1.ConsoleError.$Properties instead.
             */
            interface IConsoleError extends doramcap.console.v1.ConsoleError.$Properties {
            }

            /** Represents a ConsoleError. */
            class ConsoleError {

                /**
                 * Constructs a new ConsoleError.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.ConsoleError.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ConsoleError code. */
                code: string;

                /** ConsoleError message. */
                message: string;

                /**
                 * Creates a new ConsoleError instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ConsoleError instance
                 */
                static create(properties: doramcap.console.v1.ConsoleError.$Shape): doramcap.console.v1.ConsoleError & doramcap.console.v1.ConsoleError.$Shape;
                static create(properties?: doramcap.console.v1.ConsoleError.$Properties): doramcap.console.v1.ConsoleError;

                /**
                 * Encodes the specified ConsoleError message. Does not implicitly {@link doramcap.console.v1.ConsoleError.verify|verify} messages.
                 * @param message ConsoleError message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.ConsoleError, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ConsoleError message, length delimited. Does not implicitly {@link doramcap.console.v1.ConsoleError.verify|verify} messages.
                 * @param message ConsoleError message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.ConsoleError, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ConsoleError message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.ConsoleError & doramcap.console.v1.ConsoleError.$Shape} ConsoleError
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.ConsoleError & doramcap.console.v1.ConsoleError.$Shape;

                /**
                 * Decodes a ConsoleError message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.ConsoleError & doramcap.console.v1.ConsoleError.$Shape} ConsoleError
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.ConsoleError & doramcap.console.v1.ConsoleError.$Shape;

                /**
                 * Creates a ConsoleError message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ConsoleError
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.ConsoleError;

                /**
                 * Creates a plain object from a ConsoleError message. Also converts values to other types if specified.
                 * @param message ConsoleError
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.ConsoleError, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ConsoleError to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ConsoleError
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ConsoleError {

                /** Properties of a ConsoleError. */
                interface $Properties {

                    /** ConsoleError code */
                    code?: (string|null);

                    /** ConsoleError message */
                    message?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a ConsoleError. */
                type $Shape = doramcap.console.v1.ConsoleError.$Properties;
            }

            /**
             * Properties of a Marker.
             * @deprecated Use doramcap.console.v1.Marker.$Properties instead.
             */
            interface IMarker extends doramcap.console.v1.Marker.$Properties {
            }

            /** Represents a Marker. */
            class Marker {

                /**
                 * Constructs a new Marker.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.Marker.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Marker version. */
                version: number;

                /** Marker runId. */
                runId: string;

                /** Marker requestKey. */
                requestKey: string;

                /** Marker phase. */
                phase: doramcap.console.v1.Phase;

                /** Marker artifactReference. */
                artifactReference: string;

                /** Marker outcome. */
                outcome?: (doramcap.console.v1.Outcome|null);

                /** Marker startAccepted. */
                startAccepted: boolean;

                /** Marker recordings. */
                recordings: doramcap.console.v1.RecordingOutput[];

                /** Marker profiles. */
                profiles: string[];

                /** Marker systemApps. */
                systemApps?: (doramcap.console.v1.SystemAppSelection|null);

                /**
                 * Creates a new Marker instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Marker instance
                 */
                static create(properties: doramcap.console.v1.Marker.$Shape): doramcap.console.v1.Marker & doramcap.console.v1.Marker.$Shape;
                static create(properties?: doramcap.console.v1.Marker.$Properties): doramcap.console.v1.Marker;

                /**
                 * Encodes the specified Marker message. Does not implicitly {@link doramcap.console.v1.Marker.verify|verify} messages.
                 * @param message Marker message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.Marker, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Marker message, length delimited. Does not implicitly {@link doramcap.console.v1.Marker.verify|verify} messages.
                 * @param message Marker message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.Marker, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Marker message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.Marker & doramcap.console.v1.Marker.$Shape} Marker
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.Marker & doramcap.console.v1.Marker.$Shape;

                /**
                 * Decodes a Marker message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.Marker & doramcap.console.v1.Marker.$Shape} Marker
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.Marker & doramcap.console.v1.Marker.$Shape;

                /**
                 * Creates a Marker message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Marker
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.Marker;

                /**
                 * Creates a plain object from a Marker message. Also converts values to other types if specified.
                 * @param message Marker
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.Marker, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Marker to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Marker
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Marker {

                /** Properties of a Marker. */
                interface $Properties {

                    /** Marker version */
                    version?: (number|null);

                    /** Marker runId */
                    runId?: (string|null);

                    /** Marker requestKey */
                    requestKey?: (string|null);

                    /** Marker phase */
                    phase?: (doramcap.console.v1.Phase|null);

                    /** Marker artifactReference */
                    artifactReference?: (string|null);

                    /** Marker outcome */
                    outcome?: (doramcap.console.v1.Outcome|null);

                    /** Marker startAccepted */
                    startAccepted?: (boolean|null);

                    /** Marker recordings */
                    recordings?: (doramcap.console.v1.RecordingOutput[]|null);

                    /** Marker profiles */
                    profiles?: (string[]|null);

                    /** Marker systemApps */
                    systemApps?: (doramcap.console.v1.SystemAppSelection|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Marker. */
                type $Shape = doramcap.console.v1.Marker.$Properties;
            }

            /**
             * Properties of a RunMetric.
             * @deprecated Use doramcap.console.v1.RunMetric.$Properties instead.
             */
            interface IRunMetric extends doramcap.console.v1.RunMetric.$Properties {
            }

            /** Represents a RunMetric. */
            class RunMetric {

                /**
                 * Constructs a new RunMetric.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.RunMetric.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** RunMetric elapsedNs. */
                elapsedNs: (number|Long);

                /** RunMetric recordingBytes. */
                recordingBytes?: (number|Long|null);

                /** RunMetric writeBytesPerSecond. */
                writeBytesPerSecond?: (number|null);

                /**
                 * Creates a new RunMetric instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns RunMetric instance
                 */
                static create(properties: doramcap.console.v1.RunMetric.$Shape): doramcap.console.v1.RunMetric & doramcap.console.v1.RunMetric.$Shape;
                static create(properties?: doramcap.console.v1.RunMetric.$Properties): doramcap.console.v1.RunMetric;

                /**
                 * Encodes the specified RunMetric message. Does not implicitly {@link doramcap.console.v1.RunMetric.verify|verify} messages.
                 * @param message RunMetric message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.RunMetric, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified RunMetric message, length delimited. Does not implicitly {@link doramcap.console.v1.RunMetric.verify|verify} messages.
                 * @param message RunMetric message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.RunMetric, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a RunMetric message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.RunMetric & doramcap.console.v1.RunMetric.$Shape} RunMetric
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.RunMetric & doramcap.console.v1.RunMetric.$Shape;

                /**
                 * Decodes a RunMetric message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.RunMetric & doramcap.console.v1.RunMetric.$Shape} RunMetric
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.RunMetric & doramcap.console.v1.RunMetric.$Shape;

                /**
                 * Creates a RunMetric message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns RunMetric
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.RunMetric;

                /**
                 * Creates a plain object from a RunMetric message. Also converts values to other types if specified.
                 * @param message RunMetric
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.RunMetric, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this RunMetric to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for RunMetric
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace RunMetric {

                /** Properties of a RunMetric. */
                interface $Properties {

                    /** RunMetric elapsedNs */
                    elapsedNs?: (number|Long|null);

                    /** RunMetric recordingBytes */
                    recordingBytes?: (number|Long|null);

                    /** RunMetric writeBytesPerSecond */
                    writeBytesPerSecond?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a RunMetric. */
                type $Shape = doramcap.console.v1.RunMetric.$Properties;
            }

            /**
             * Properties of a StreamCursor.
             * @deprecated Use doramcap.console.v1.StreamCursor.$Properties instead.
             */
            interface IStreamCursor extends doramcap.console.v1.StreamCursor.$Properties {
            }

            /** Represents a StreamCursor. */
            class StreamCursor {

                /**
                 * Constructs a new StreamCursor.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.StreamCursor.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** StreamCursor runId. */
                runId: string;

                /** StreamCursor sequence. */
                sequence: (number|Long);

                /** StreamCursor metricElapsedNs. */
                metricElapsedNs: (number|Long);

                /**
                 * Creates a new StreamCursor instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns StreamCursor instance
                 */
                static create(properties: doramcap.console.v1.StreamCursor.$Shape): doramcap.console.v1.StreamCursor & doramcap.console.v1.StreamCursor.$Shape;
                static create(properties?: doramcap.console.v1.StreamCursor.$Properties): doramcap.console.v1.StreamCursor;

                /**
                 * Encodes the specified StreamCursor message. Does not implicitly {@link doramcap.console.v1.StreamCursor.verify|verify} messages.
                 * @param message StreamCursor message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.StreamCursor, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified StreamCursor message, length delimited. Does not implicitly {@link doramcap.console.v1.StreamCursor.verify|verify} messages.
                 * @param message StreamCursor message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.StreamCursor, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a StreamCursor message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.StreamCursor & doramcap.console.v1.StreamCursor.$Shape} StreamCursor
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.StreamCursor & doramcap.console.v1.StreamCursor.$Shape;

                /**
                 * Decodes a StreamCursor message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.StreamCursor & doramcap.console.v1.StreamCursor.$Shape} StreamCursor
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.StreamCursor & doramcap.console.v1.StreamCursor.$Shape;

                /**
                 * Creates a StreamCursor message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns StreamCursor
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.StreamCursor;

                /**
                 * Creates a plain object from a StreamCursor message. Also converts values to other types if specified.
                 * @param message StreamCursor
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.StreamCursor, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this StreamCursor to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for StreamCursor
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace StreamCursor {

                /** Properties of a StreamCursor. */
                interface $Properties {

                    /** StreamCursor runId */
                    runId?: (string|null);

                    /** StreamCursor sequence */
                    sequence?: (number|Long|null);

                    /** StreamCursor metricElapsedNs */
                    metricElapsedNs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a StreamCursor. */
                type $Shape = doramcap.console.v1.StreamCursor.$Properties;
            }

            /** HistoryState enum. */
            enum HistoryState {

                /** HISTORY_STATE_UNSPECIFIED value */
                HISTORY_STATE_UNSPECIFIED = 0,

                /** HISTORY_RUNNING value */
                HISTORY_RUNNING = 1,

                /** HISTORY_COMPLETED value */
                HISTORY_COMPLETED = 2,

                /** HISTORY_FAILED value */
                HISTORY_FAILED = 3,

                /** HISTORY_CANCELLED value */
                HISTORY_CANCELLED = 4,

                /** HISTORY_BLOCKED value */
                HISTORY_BLOCKED = 5,

                /** HISTORY_INTERRUPTED value */
                HISTORY_INTERRUPTED = 6
            }

            /**
             * Properties of a HistoryArtifact.
             * @deprecated Use doramcap.console.v1.HistoryArtifact.$Properties instead.
             */
            interface IHistoryArtifact extends doramcap.console.v1.HistoryArtifact.$Properties {
            }

            /** Represents a HistoryArtifact. */
            class HistoryArtifact {

                /**
                 * Constructs a new HistoryArtifact.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.HistoryArtifact.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** HistoryArtifact id. */
                id: string;

                /** HistoryArtifact sizeBytes. */
                sizeBytes?: (number|Long|null);

                /** HistoryArtifact downloadable. */
                downloadable: boolean;

                /**
                 * Creates a new HistoryArtifact instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns HistoryArtifact instance
                 */
                static create(properties: doramcap.console.v1.HistoryArtifact.$Shape): doramcap.console.v1.HistoryArtifact & doramcap.console.v1.HistoryArtifact.$Shape;
                static create(properties?: doramcap.console.v1.HistoryArtifact.$Properties): doramcap.console.v1.HistoryArtifact;

                /**
                 * Encodes the specified HistoryArtifact message. Does not implicitly {@link doramcap.console.v1.HistoryArtifact.verify|verify} messages.
                 * @param message HistoryArtifact message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.HistoryArtifact, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified HistoryArtifact message, length delimited. Does not implicitly {@link doramcap.console.v1.HistoryArtifact.verify|verify} messages.
                 * @param message HistoryArtifact message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.HistoryArtifact, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a HistoryArtifact message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.HistoryArtifact & doramcap.console.v1.HistoryArtifact.$Shape} HistoryArtifact
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.HistoryArtifact & doramcap.console.v1.HistoryArtifact.$Shape;

                /**
                 * Decodes a HistoryArtifact message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.HistoryArtifact & doramcap.console.v1.HistoryArtifact.$Shape} HistoryArtifact
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.HistoryArtifact & doramcap.console.v1.HistoryArtifact.$Shape;

                /**
                 * Creates a HistoryArtifact message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns HistoryArtifact
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.HistoryArtifact;

                /**
                 * Creates a plain object from a HistoryArtifact message. Also converts values to other types if specified.
                 * @param message HistoryArtifact
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.HistoryArtifact, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this HistoryArtifact to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for HistoryArtifact
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace HistoryArtifact {

                /** Properties of a HistoryArtifact. */
                interface $Properties {

                    /** HistoryArtifact id */
                    id?: (string|null);

                    /** HistoryArtifact sizeBytes */
                    sizeBytes?: (number|Long|null);

                    /** HistoryArtifact downloadable */
                    downloadable?: (boolean|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a HistoryArtifact. */
                type $Shape = doramcap.console.v1.HistoryArtifact.$Properties;
            }

            /**
             * Properties of a HistoryEntry.
             * @deprecated Use doramcap.console.v1.HistoryEntry.$Properties instead.
             */
            interface IHistoryEntry extends doramcap.console.v1.HistoryEntry.$Properties {
            }

            /** Represents a HistoryEntry. */
            class HistoryEntry {

                /**
                 * Constructs a new HistoryEntry.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.HistoryEntry.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** HistoryEntry runId. */
                runId: string;

                /** HistoryEntry profiles. */
                profiles: string[];

                /** HistoryEntry systemApps. */
                systemApps?: (doramcap.console.v1.SystemAppSelection|null);

                /** HistoryEntry startedAtMs. */
                startedAtMs: (number|Long);

                /** HistoryEntry finishedAtMs. */
                finishedAtMs?: (number|Long|null);

                /** HistoryEntry state. */
                state: doramcap.console.v1.HistoryState;

                /** HistoryEntry errorSummary. */
                errorSummary: string;

                /** HistoryEntry artifacts. */
                artifacts: doramcap.console.v1.HistoryArtifact[];

                /**
                 * Creates a new HistoryEntry instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns HistoryEntry instance
                 */
                static create(properties: doramcap.console.v1.HistoryEntry.$Shape): doramcap.console.v1.HistoryEntry & doramcap.console.v1.HistoryEntry.$Shape;
                static create(properties?: doramcap.console.v1.HistoryEntry.$Properties): doramcap.console.v1.HistoryEntry;

                /**
                 * Encodes the specified HistoryEntry message. Does not implicitly {@link doramcap.console.v1.HistoryEntry.verify|verify} messages.
                 * @param message HistoryEntry message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.HistoryEntry, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified HistoryEntry message, length delimited. Does not implicitly {@link doramcap.console.v1.HistoryEntry.verify|verify} messages.
                 * @param message HistoryEntry message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.HistoryEntry, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a HistoryEntry message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.HistoryEntry & doramcap.console.v1.HistoryEntry.$Shape} HistoryEntry
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.HistoryEntry & doramcap.console.v1.HistoryEntry.$Shape;

                /**
                 * Decodes a HistoryEntry message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.HistoryEntry & doramcap.console.v1.HistoryEntry.$Shape} HistoryEntry
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.HistoryEntry & doramcap.console.v1.HistoryEntry.$Shape;

                /**
                 * Creates a HistoryEntry message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns HistoryEntry
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.HistoryEntry;

                /**
                 * Creates a plain object from a HistoryEntry message. Also converts values to other types if specified.
                 * @param message HistoryEntry
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.HistoryEntry, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this HistoryEntry to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for HistoryEntry
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace HistoryEntry {

                /** Properties of a HistoryEntry. */
                interface $Properties {

                    /** HistoryEntry runId */
                    runId?: (string|null);

                    /** HistoryEntry profiles */
                    profiles?: (string[]|null);

                    /** HistoryEntry systemApps */
                    systemApps?: (doramcap.console.v1.SystemAppSelection|null);

                    /** HistoryEntry startedAtMs */
                    startedAtMs?: (number|Long|null);

                    /** HistoryEntry finishedAtMs */
                    finishedAtMs?: (number|Long|null);

                    /** HistoryEntry state */
                    state?: (doramcap.console.v1.HistoryState|null);

                    /** HistoryEntry errorSummary */
                    errorSummary?: (string|null);

                    /** HistoryEntry artifacts */
                    artifacts?: (doramcap.console.v1.HistoryArtifact[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a HistoryEntry. */
                type $Shape = doramcap.console.v1.HistoryEntry.$Properties;
            }

            /**
             * Properties of a HistoryQuery.
             * @deprecated Use doramcap.console.v1.HistoryQuery.$Properties instead.
             */
            interface IHistoryQuery extends doramcap.console.v1.HistoryQuery.$Properties {
            }

            /** Represents a HistoryQuery. */
            class HistoryQuery {

                /**
                 * Constructs a new HistoryQuery.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.HistoryQuery.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** HistoryQuery limit. */
                limit: number;

                /** HistoryQuery cursor. */
                cursor: string;

                /** HistoryQuery state. */
                state: doramcap.console.v1.HistoryState;

                /**
                 * Creates a new HistoryQuery instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns HistoryQuery instance
                 */
                static create(properties: doramcap.console.v1.HistoryQuery.$Shape): doramcap.console.v1.HistoryQuery & doramcap.console.v1.HistoryQuery.$Shape;
                static create(properties?: doramcap.console.v1.HistoryQuery.$Properties): doramcap.console.v1.HistoryQuery;

                /**
                 * Encodes the specified HistoryQuery message. Does not implicitly {@link doramcap.console.v1.HistoryQuery.verify|verify} messages.
                 * @param message HistoryQuery message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.HistoryQuery, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified HistoryQuery message, length delimited. Does not implicitly {@link doramcap.console.v1.HistoryQuery.verify|verify} messages.
                 * @param message HistoryQuery message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.HistoryQuery, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a HistoryQuery message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.HistoryQuery & doramcap.console.v1.HistoryQuery.$Shape} HistoryQuery
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.HistoryQuery & doramcap.console.v1.HistoryQuery.$Shape;

                /**
                 * Decodes a HistoryQuery message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.HistoryQuery & doramcap.console.v1.HistoryQuery.$Shape} HistoryQuery
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.HistoryQuery & doramcap.console.v1.HistoryQuery.$Shape;

                /**
                 * Creates a HistoryQuery message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns HistoryQuery
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.HistoryQuery;

                /**
                 * Creates a plain object from a HistoryQuery message. Also converts values to other types if specified.
                 * @param message HistoryQuery
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.HistoryQuery, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this HistoryQuery to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for HistoryQuery
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace HistoryQuery {

                /** Properties of a HistoryQuery. */
                interface $Properties {

                    /** HistoryQuery limit */
                    limit?: (number|null);

                    /** HistoryQuery cursor */
                    cursor?: (string|null);

                    /** HistoryQuery state */
                    state?: (doramcap.console.v1.HistoryState|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a HistoryQuery. */
                type $Shape = doramcap.console.v1.HistoryQuery.$Properties;
            }

            /**
             * Properties of a HistoryPage.
             * @deprecated Use doramcap.console.v1.HistoryPage.$Properties instead.
             */
            interface IHistoryPage extends doramcap.console.v1.HistoryPage.$Properties {
            }

            /** Represents a HistoryPage. */
            class HistoryPage {

                /**
                 * Constructs a new HistoryPage.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.HistoryPage.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** HistoryPage entries. */
                entries: doramcap.console.v1.HistoryEntry[];

                /** HistoryPage nextCursor. */
                nextCursor: string;

                /**
                 * Creates a new HistoryPage instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns HistoryPage instance
                 */
                static create(properties: doramcap.console.v1.HistoryPage.$Shape): doramcap.console.v1.HistoryPage & doramcap.console.v1.HistoryPage.$Shape;
                static create(properties?: doramcap.console.v1.HistoryPage.$Properties): doramcap.console.v1.HistoryPage;

                /**
                 * Encodes the specified HistoryPage message. Does not implicitly {@link doramcap.console.v1.HistoryPage.verify|verify} messages.
                 * @param message HistoryPage message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.HistoryPage, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified HistoryPage message, length delimited. Does not implicitly {@link doramcap.console.v1.HistoryPage.verify|verify} messages.
                 * @param message HistoryPage message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.HistoryPage, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a HistoryPage message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.HistoryPage & doramcap.console.v1.HistoryPage.$Shape} HistoryPage
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.HistoryPage & doramcap.console.v1.HistoryPage.$Shape;

                /**
                 * Decodes a HistoryPage message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.HistoryPage & doramcap.console.v1.HistoryPage.$Shape} HistoryPage
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.HistoryPage & doramcap.console.v1.HistoryPage.$Shape;

                /**
                 * Creates a HistoryPage message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns HistoryPage
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.HistoryPage;

                /**
                 * Creates a plain object from a HistoryPage message. Also converts values to other types if specified.
                 * @param message HistoryPage
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.HistoryPage, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this HistoryPage to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for HistoryPage
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace HistoryPage {

                /** Properties of a HistoryPage. */
                interface $Properties {

                    /** HistoryPage entries */
                    entries?: (doramcap.console.v1.HistoryEntry[]|null);

                    /** HistoryPage nextCursor */
                    nextCursor?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a HistoryPage. */
                type $Shape = doramcap.console.v1.HistoryPage.$Properties;
            }

            /**
             * Properties of a HistoryDetail.
             * @deprecated Use doramcap.console.v1.HistoryDetail.$Properties instead.
             */
            interface IHistoryDetail extends doramcap.console.v1.HistoryDetail.$Properties {
            }

            /** Represents a HistoryDetail. */
            class HistoryDetail {

                /**
                 * Constructs a new HistoryDetail.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.HistoryDetail.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** HistoryDetail entry. */
                entry?: (doramcap.console.v1.HistoryEntry|null);

                /** HistoryDetail status. */
                status?: (doramcap.console.v1.Status|null);

                /**
                 * Creates a new HistoryDetail instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns HistoryDetail instance
                 */
                static create(properties: doramcap.console.v1.HistoryDetail.$Shape): doramcap.console.v1.HistoryDetail & doramcap.console.v1.HistoryDetail.$Shape;
                static create(properties?: doramcap.console.v1.HistoryDetail.$Properties): doramcap.console.v1.HistoryDetail;

                /**
                 * Encodes the specified HistoryDetail message. Does not implicitly {@link doramcap.console.v1.HistoryDetail.verify|verify} messages.
                 * @param message HistoryDetail message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.HistoryDetail, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified HistoryDetail message, length delimited. Does not implicitly {@link doramcap.console.v1.HistoryDetail.verify|verify} messages.
                 * @param message HistoryDetail message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.HistoryDetail, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a HistoryDetail message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.HistoryDetail & doramcap.console.v1.HistoryDetail.$Shape} HistoryDetail
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.HistoryDetail & doramcap.console.v1.HistoryDetail.$Shape;

                /**
                 * Decodes a HistoryDetail message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.HistoryDetail & doramcap.console.v1.HistoryDetail.$Shape} HistoryDetail
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.HistoryDetail & doramcap.console.v1.HistoryDetail.$Shape;

                /**
                 * Creates a HistoryDetail message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns HistoryDetail
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.HistoryDetail;

                /**
                 * Creates a plain object from a HistoryDetail message. Also converts values to other types if specified.
                 * @param message HistoryDetail
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.HistoryDetail, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this HistoryDetail to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for HistoryDetail
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace HistoryDetail {

                /** Properties of a HistoryDetail. */
                interface $Properties {

                    /** HistoryDetail entry */
                    entry?: (doramcap.console.v1.HistoryEntry|null);

                    /** HistoryDetail status */
                    status?: (doramcap.console.v1.Status|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a HistoryDetail. */
                type $Shape = doramcap.console.v1.HistoryDetail.$Properties;
            }

            /**
             * Properties of a ConsoleConfig.
             * @deprecated Use doramcap.console.v1.ConsoleConfig.$Properties instead.
             */
            interface IConsoleConfig extends doramcap.console.v1.ConsoleConfig.$Properties {
            }

            /** Represents a ConsoleConfig. */
            class ConsoleConfig {

                /**
                 * Constructs a new ConsoleConfig.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.ConsoleConfig.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ConsoleConfig downloadMissingImages. */
                downloadMissingImages: boolean;

                /** ConsoleConfig listenAddress. */
                listenAddress: string;

                /** ConsoleConfig projectRoot. */
                projectRoot: string;

                /** ConsoleConfig dataRoot. */
                dataRoot: string;

                /** ConsoleConfig dockerSocket. */
                dockerSocket: string;

                /** ConsoleConfig accessSecretFile. */
                accessSecretFile: string;

                /** ConsoleConfig systemApps. */
                systemApps: doramcap.console.v1.SystemAppDefinition[];

                /** ConsoleConfig dora. */
                dora?: (doramcap.dora.v1.DoraRuntime|null);

                /** ConsoleConfig publicOrigin. */
                publicOrigin: string;

                /** ConsoleConfig runtime. */
                runtime?: (doramcap.console.v1.RuntimeConfig|null);

                /** ConsoleConfig liveBridge. */
                liveBridge?: (doramcap.console.v1.LiveBridge|null);

                /**
                 * Creates a new ConsoleConfig instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ConsoleConfig instance
                 */
                static create(properties: doramcap.console.v1.ConsoleConfig.$Shape): doramcap.console.v1.ConsoleConfig & doramcap.console.v1.ConsoleConfig.$Shape;
                static create(properties?: doramcap.console.v1.ConsoleConfig.$Properties): doramcap.console.v1.ConsoleConfig;

                /**
                 * Encodes the specified ConsoleConfig message. Does not implicitly {@link doramcap.console.v1.ConsoleConfig.verify|verify} messages.
                 * @param message ConsoleConfig message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.ConsoleConfig, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ConsoleConfig message, length delimited. Does not implicitly {@link doramcap.console.v1.ConsoleConfig.verify|verify} messages.
                 * @param message ConsoleConfig message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.ConsoleConfig, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ConsoleConfig message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.ConsoleConfig & doramcap.console.v1.ConsoleConfig.$Shape} ConsoleConfig
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.ConsoleConfig & doramcap.console.v1.ConsoleConfig.$Shape;

                /**
                 * Decodes a ConsoleConfig message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.ConsoleConfig & doramcap.console.v1.ConsoleConfig.$Shape} ConsoleConfig
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.ConsoleConfig & doramcap.console.v1.ConsoleConfig.$Shape;

                /**
                 * Creates a ConsoleConfig message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ConsoleConfig
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.ConsoleConfig;

                /**
                 * Creates a plain object from a ConsoleConfig message. Also converts values to other types if specified.
                 * @param message ConsoleConfig
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.ConsoleConfig, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ConsoleConfig to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ConsoleConfig
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ConsoleConfig {

                /** Properties of a ConsoleConfig. */
                interface $Properties {

                    /** ConsoleConfig downloadMissingImages */
                    downloadMissingImages?: (boolean|null);

                    /** ConsoleConfig listenAddress */
                    listenAddress?: (string|null);

                    /** ConsoleConfig projectRoot */
                    projectRoot?: (string|null);

                    /** ConsoleConfig dataRoot */
                    dataRoot?: (string|null);

                    /** ConsoleConfig dockerSocket */
                    dockerSocket?: (string|null);

                    /** ConsoleConfig accessSecretFile */
                    accessSecretFile?: (string|null);

                    /** ConsoleConfig systemApps */
                    systemApps?: (doramcap.console.v1.SystemAppDefinition[]|null);

                    /** ConsoleConfig dora */
                    dora?: (doramcap.dora.v1.DoraRuntime|null);

                    /** ConsoleConfig publicOrigin */
                    publicOrigin?: (string|null);

                    /** ConsoleConfig runtime */
                    runtime?: (doramcap.console.v1.RuntimeConfig|null);

                    /** ConsoleConfig liveBridge */
                    liveBridge?: (doramcap.console.v1.LiveBridge|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a ConsoleConfig. */
                type $Shape = doramcap.console.v1.ConsoleConfig.$Properties;
            }

            /**
             * Properties of a RuntimeConfig.
             * @deprecated Use doramcap.console.v1.RuntimeConfig.$Properties instead.
             */
            interface IRuntimeConfig extends doramcap.console.v1.RuntimeConfig.$Properties {
            }

            /** Represents a RuntimeConfig. */
            class RuntimeConfig {

                /**
                 * Constructs a new RuntimeConfig.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.RuntimeConfig.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** RuntimeConfig readinessSeconds. */
                readinessSeconds?: (number|null);

                /** RuntimeConfig stopSeconds. */
                stopSeconds?: (number|null);

                /** RuntimeConfig sampleIntervalMs. */
                sampleIntervalMs?: (number|null);

                /**
                 * Creates a new RuntimeConfig instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns RuntimeConfig instance
                 */
                static create(properties: doramcap.console.v1.RuntimeConfig.$Shape): doramcap.console.v1.RuntimeConfig & doramcap.console.v1.RuntimeConfig.$Shape;
                static create(properties?: doramcap.console.v1.RuntimeConfig.$Properties): doramcap.console.v1.RuntimeConfig;

                /**
                 * Encodes the specified RuntimeConfig message. Does not implicitly {@link doramcap.console.v1.RuntimeConfig.verify|verify} messages.
                 * @param message RuntimeConfig message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.RuntimeConfig, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified RuntimeConfig message, length delimited. Does not implicitly {@link doramcap.console.v1.RuntimeConfig.verify|verify} messages.
                 * @param message RuntimeConfig message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.RuntimeConfig, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a RuntimeConfig message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.RuntimeConfig & doramcap.console.v1.RuntimeConfig.$Shape} RuntimeConfig
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.RuntimeConfig & doramcap.console.v1.RuntimeConfig.$Shape;

                /**
                 * Decodes a RuntimeConfig message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.RuntimeConfig & doramcap.console.v1.RuntimeConfig.$Shape} RuntimeConfig
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.RuntimeConfig & doramcap.console.v1.RuntimeConfig.$Shape;

                /**
                 * Creates a RuntimeConfig message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns RuntimeConfig
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.RuntimeConfig;

                /**
                 * Creates a plain object from a RuntimeConfig message. Also converts values to other types if specified.
                 * @param message RuntimeConfig
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.RuntimeConfig, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this RuntimeConfig to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for RuntimeConfig
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace RuntimeConfig {

                /** Properties of a RuntimeConfig. */
                interface $Properties {

                    /** RuntimeConfig readinessSeconds */
                    readinessSeconds?: (number|null);

                    /** RuntimeConfig stopSeconds */
                    stopSeconds?: (number|null);

                    /** RuntimeConfig sampleIntervalMs */
                    sampleIntervalMs?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a RuntimeConfig. */
                type $Shape = doramcap.console.v1.RuntimeConfig.$Properties;
            }

            /**
             * Properties of an OutputMount.
             * @deprecated Use doramcap.console.v1.OutputMount.$Properties instead.
             */
            interface IOutputMount extends doramcap.console.v1.OutputMount.$Properties {
            }

            /** Represents an OutputMount. */
            class OutputMount {

                /**
                 * Constructs a new OutputMount.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.OutputMount.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** OutputMount service. */
                service: string;

                /** OutputMount target. */
                target: string;

                /**
                 * Creates a new OutputMount instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns OutputMount instance
                 */
                static create(properties: doramcap.console.v1.OutputMount.$Shape): doramcap.console.v1.OutputMount & doramcap.console.v1.OutputMount.$Shape;
                static create(properties?: doramcap.console.v1.OutputMount.$Properties): doramcap.console.v1.OutputMount;

                /**
                 * Encodes the specified OutputMount message. Does not implicitly {@link doramcap.console.v1.OutputMount.verify|verify} messages.
                 * @param message OutputMount message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.OutputMount, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified OutputMount message, length delimited. Does not implicitly {@link doramcap.console.v1.OutputMount.verify|verify} messages.
                 * @param message OutputMount message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.OutputMount, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an OutputMount message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.OutputMount & doramcap.console.v1.OutputMount.$Shape} OutputMount
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.OutputMount & doramcap.console.v1.OutputMount.$Shape;

                /**
                 * Decodes an OutputMount message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.OutputMount & doramcap.console.v1.OutputMount.$Shape} OutputMount
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.OutputMount & doramcap.console.v1.OutputMount.$Shape;

                /**
                 * Creates an OutputMount message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns OutputMount
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.OutputMount;

                /**
                 * Creates a plain object from an OutputMount message. Also converts values to other types if specified.
                 * @param message OutputMount
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.OutputMount, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this OutputMount to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for OutputMount
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace OutputMount {

                /** Properties of an OutputMount. */
                interface $Properties {

                    /** OutputMount service */
                    service?: (string|null);

                    /** OutputMount target */
                    target?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an OutputMount. */
                type $Shape = doramcap.console.v1.OutputMount.$Properties;
            }

            /**
             * Properties of a LaunchProfile.
             * @deprecated Use doramcap.console.v1.LaunchProfile.$Properties instead.
             */
            interface ILaunchProfile extends doramcap.console.v1.LaunchProfile.$Properties {
            }

            /** Represents a LaunchProfile. */
            class LaunchProfile {

                /**
                 * Constructs a new LaunchProfile.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.LaunchProfile.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** LaunchProfile id. */
                id: string;

                /** LaunchProfile label. */
                label: string;

                /** LaunchProfile config. */
                config: string;

                /** LaunchProfile recordings. */
                recordings: doramcap.console.v1.RecordingOutput[];

                /** LaunchProfile liveOutputs. */
                liveOutputs: doramcap.console.v1.LiveOutput[];

                /**
                 * Creates a new LaunchProfile instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns LaunchProfile instance
                 */
                static create(properties: doramcap.console.v1.LaunchProfile.$Shape): doramcap.console.v1.LaunchProfile & doramcap.console.v1.LaunchProfile.$Shape;
                static create(properties?: doramcap.console.v1.LaunchProfile.$Properties): doramcap.console.v1.LaunchProfile;

                /**
                 * Encodes the specified LaunchProfile message. Does not implicitly {@link doramcap.console.v1.LaunchProfile.verify|verify} messages.
                 * @param message LaunchProfile message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.LaunchProfile, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified LaunchProfile message, length delimited. Does not implicitly {@link doramcap.console.v1.LaunchProfile.verify|verify} messages.
                 * @param message LaunchProfile message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.LaunchProfile, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a LaunchProfile message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.LaunchProfile & doramcap.console.v1.LaunchProfile.$Shape} LaunchProfile
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.LaunchProfile & doramcap.console.v1.LaunchProfile.$Shape;

                /**
                 * Decodes a LaunchProfile message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.LaunchProfile & doramcap.console.v1.LaunchProfile.$Shape} LaunchProfile
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.LaunchProfile & doramcap.console.v1.LaunchProfile.$Shape;

                /**
                 * Creates a LaunchProfile message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns LaunchProfile
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.LaunchProfile;

                /**
                 * Creates a plain object from a LaunchProfile message. Also converts values to other types if specified.
                 * @param message LaunchProfile
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.LaunchProfile, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this LaunchProfile to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for LaunchProfile
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace LaunchProfile {

                /** Properties of a LaunchProfile. */
                interface $Properties {

                    /** LaunchProfile id */
                    id?: (string|null);

                    /** LaunchProfile label */
                    label?: (string|null);

                    /** LaunchProfile config */
                    config?: (string|null);

                    /** LaunchProfile recordings */
                    recordings?: (doramcap.console.v1.RecordingOutput[]|null);

                    /** LaunchProfile liveOutputs */
                    liveOutputs?: (doramcap.console.v1.LiveOutput[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a LaunchProfile. */
                type $Shape = doramcap.console.v1.LaunchProfile.$Properties;
            }

            /**
             * Properties of a SystemAppDefinition.
             * @deprecated Use doramcap.console.v1.SystemAppDefinition.$Properties instead.
             */
            interface ISystemAppDefinition extends doramcap.console.v1.SystemAppDefinition.$Properties {
            }

            /** Represents a SystemAppDefinition. */
            class SystemAppDefinition {

                /**
                 * Constructs a new SystemAppDefinition.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.SystemAppDefinition.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** SystemAppDefinition id. */
                id: string;

                /** SystemAppDefinition label. */
                label: string;

                /** SystemAppDefinition defaultSelected. */
                defaultSelected: boolean;

                /** SystemAppDefinition input. */
                input?: (doramcap.compose.v1.ComposeProject|null);

                /** SystemAppDefinition recordings. */
                recordings: doramcap.console.v1.RecordingOutput[];

                /** SystemAppDefinition graph. */
                graph: string;

                /** SystemAppDefinition outputMounts. */
                outputMounts: doramcap.console.v1.OutputMount[];

                /** SystemAppDefinition liveOutputs. */
                liveOutputs: doramcap.console.v1.LiveOutput[];

                /** SystemAppDefinition configMount. */
                configMount?: (doramcap.console.v1.OutputMount|null);

                /** SystemAppDefinition launchProfiles. */
                launchProfiles: doramcap.console.v1.LaunchProfile[];

                /**
                 * Creates a new SystemAppDefinition instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns SystemAppDefinition instance
                 */
                static create(properties: doramcap.console.v1.SystemAppDefinition.$Shape): doramcap.console.v1.SystemAppDefinition & doramcap.console.v1.SystemAppDefinition.$Shape;
                static create(properties?: doramcap.console.v1.SystemAppDefinition.$Properties): doramcap.console.v1.SystemAppDefinition;

                /**
                 * Encodes the specified SystemAppDefinition message. Does not implicitly {@link doramcap.console.v1.SystemAppDefinition.verify|verify} messages.
                 * @param message SystemAppDefinition message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.SystemAppDefinition, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified SystemAppDefinition message, length delimited. Does not implicitly {@link doramcap.console.v1.SystemAppDefinition.verify|verify} messages.
                 * @param message SystemAppDefinition message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.SystemAppDefinition, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a SystemAppDefinition message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.SystemAppDefinition & doramcap.console.v1.SystemAppDefinition.$Shape} SystemAppDefinition
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.SystemAppDefinition & doramcap.console.v1.SystemAppDefinition.$Shape;

                /**
                 * Decodes a SystemAppDefinition message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.SystemAppDefinition & doramcap.console.v1.SystemAppDefinition.$Shape} SystemAppDefinition
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.SystemAppDefinition & doramcap.console.v1.SystemAppDefinition.$Shape;

                /**
                 * Creates a SystemAppDefinition message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns SystemAppDefinition
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.SystemAppDefinition;

                /**
                 * Creates a plain object from a SystemAppDefinition message. Also converts values to other types if specified.
                 * @param message SystemAppDefinition
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.SystemAppDefinition, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this SystemAppDefinition to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for SystemAppDefinition
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace SystemAppDefinition {

                /** Properties of a SystemAppDefinition. */
                interface $Properties {

                    /** SystemAppDefinition id */
                    id?: (string|null);

                    /** SystemAppDefinition label */
                    label?: (string|null);

                    /** SystemAppDefinition defaultSelected */
                    defaultSelected?: (boolean|null);

                    /** SystemAppDefinition input */
                    input?: (doramcap.compose.v1.ComposeProject|null);

                    /** SystemAppDefinition recordings */
                    recordings?: (doramcap.console.v1.RecordingOutput[]|null);

                    /** SystemAppDefinition graph */
                    graph?: (string|null);

                    /** SystemAppDefinition outputMounts */
                    outputMounts?: (doramcap.console.v1.OutputMount[]|null);

                    /** SystemAppDefinition liveOutputs */
                    liveOutputs?: (doramcap.console.v1.LiveOutput[]|null);

                    /** SystemAppDefinition configMount */
                    configMount?: (doramcap.console.v1.OutputMount|null);

                    /** SystemAppDefinition launchProfiles */
                    launchProfiles?: (doramcap.console.v1.LaunchProfile[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a SystemAppDefinition. */
                type $Shape = doramcap.console.v1.SystemAppDefinition.$Properties;
            }

            /**
             * Properties of a LiveBridge.
             * @deprecated Use doramcap.console.v1.LiveBridge.$Properties instead.
             */
            interface ILiveBridge extends doramcap.console.v1.LiveBridge.$Properties {
            }

            /** Represents a LiveBridge. */
            class LiveBridge {

                /**
                 * Constructs a new LiveBridge.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.LiveBridge.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** LiveBridge image. */
                image: string;

                /** LiveBridge port. */
                port: number;

                /**
                 * Creates a new LiveBridge instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns LiveBridge instance
                 */
                static create(properties: doramcap.console.v1.LiveBridge.$Shape): doramcap.console.v1.LiveBridge & doramcap.console.v1.LiveBridge.$Shape;
                static create(properties?: doramcap.console.v1.LiveBridge.$Properties): doramcap.console.v1.LiveBridge;

                /**
                 * Encodes the specified LiveBridge message. Does not implicitly {@link doramcap.console.v1.LiveBridge.verify|verify} messages.
                 * @param message LiveBridge message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.LiveBridge, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified LiveBridge message, length delimited. Does not implicitly {@link doramcap.console.v1.LiveBridge.verify|verify} messages.
                 * @param message LiveBridge message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.LiveBridge, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a LiveBridge message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.LiveBridge & doramcap.console.v1.LiveBridge.$Shape} LiveBridge
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.LiveBridge & doramcap.console.v1.LiveBridge.$Shape;

                /**
                 * Decodes a LiveBridge message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.LiveBridge & doramcap.console.v1.LiveBridge.$Shape} LiveBridge
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.LiveBridge & doramcap.console.v1.LiveBridge.$Shape;

                /**
                 * Creates a LiveBridge message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns LiveBridge
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.LiveBridge;

                /**
                 * Creates a plain object from a LiveBridge message. Also converts values to other types if specified.
                 * @param message LiveBridge
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.LiveBridge, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this LiveBridge to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for LiveBridge
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace LiveBridge {

                /** Properties of a LiveBridge. */
                interface $Properties {

                    /** LiveBridge image */
                    image?: (string|null);

                    /** LiveBridge port */
                    port?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a LiveBridge. */
                type $Shape = doramcap.console.v1.LiveBridge.$Properties;
            }

            /**
             * Properties of a LiveOutput.
             * @deprecated Use doramcap.console.v1.LiveOutput.$Properties instead.
             */
            interface ILiveOutput extends doramcap.console.v1.LiveOutput.$Properties {
            }

            /** Represents a LiveOutput. */
            class LiveOutput {

                /**
                 * Constructs a new LiveOutput.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.LiveOutput.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** LiveOutput source. */
                source: string;

                /** LiveOutput kind. */
                kind: doramcap.console.live.v1.Kind;

                /**
                 * Creates a new LiveOutput instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns LiveOutput instance
                 */
                static create(properties: doramcap.console.v1.LiveOutput.$Shape): doramcap.console.v1.LiveOutput & doramcap.console.v1.LiveOutput.$Shape;
                static create(properties?: doramcap.console.v1.LiveOutput.$Properties): doramcap.console.v1.LiveOutput;

                /**
                 * Encodes the specified LiveOutput message. Does not implicitly {@link doramcap.console.v1.LiveOutput.verify|verify} messages.
                 * @param message LiveOutput message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.LiveOutput, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified LiveOutput message, length delimited. Does not implicitly {@link doramcap.console.v1.LiveOutput.verify|verify} messages.
                 * @param message LiveOutput message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.LiveOutput, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a LiveOutput message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.LiveOutput & doramcap.console.v1.LiveOutput.$Shape} LiveOutput
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.LiveOutput & doramcap.console.v1.LiveOutput.$Shape;

                /**
                 * Decodes a LiveOutput message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.LiveOutput & doramcap.console.v1.LiveOutput.$Shape} LiveOutput
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.LiveOutput & doramcap.console.v1.LiveOutput.$Shape;

                /**
                 * Creates a LiveOutput message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns LiveOutput
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.LiveOutput;

                /**
                 * Creates a plain object from a LiveOutput message. Also converts values to other types if specified.
                 * @param message LiveOutput
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.LiveOutput, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this LiveOutput to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for LiveOutput
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace LiveOutput {

                /** Properties of a LiveOutput. */
                interface $Properties {

                    /** LiveOutput source */
                    source?: (string|null);

                    /** LiveOutput kind */
                    kind?: (doramcap.console.live.v1.Kind|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a LiveOutput. */
                type $Shape = doramcap.console.v1.LiveOutput.$Properties;
            }

            /**
             * Properties of a RecordingOutput.
             * @deprecated Use doramcap.console.v1.RecordingOutput.$Properties instead.
             */
            interface IRecordingOutput extends doramcap.console.v1.RecordingOutput.$Properties {
            }

            /** Represents a RecordingOutput. */
            class RecordingOutput {

                /**
                 * Constructs a new RecordingOutput.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.RecordingOutput.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** RecordingOutput id. */
                id: string;

                /** RecordingOutput path. */
                path: string;

                /** RecordingOutput stream. */
                stream: string;

                /** RecordingOutput cpu. */
                cpu: boolean;

                /**
                 * Creates a new RecordingOutput instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns RecordingOutput instance
                 */
                static create(properties: doramcap.console.v1.RecordingOutput.$Shape): doramcap.console.v1.RecordingOutput & doramcap.console.v1.RecordingOutput.$Shape;
                static create(properties?: doramcap.console.v1.RecordingOutput.$Properties): doramcap.console.v1.RecordingOutput;

                /**
                 * Encodes the specified RecordingOutput message. Does not implicitly {@link doramcap.console.v1.RecordingOutput.verify|verify} messages.
                 * @param message RecordingOutput message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.RecordingOutput, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified RecordingOutput message, length delimited. Does not implicitly {@link doramcap.console.v1.RecordingOutput.verify|verify} messages.
                 * @param message RecordingOutput message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.RecordingOutput, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a RecordingOutput message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.RecordingOutput & doramcap.console.v1.RecordingOutput.$Shape} RecordingOutput
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.RecordingOutput & doramcap.console.v1.RecordingOutput.$Shape;

                /**
                 * Decodes a RecordingOutput message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.RecordingOutput & doramcap.console.v1.RecordingOutput.$Shape} RecordingOutput
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.RecordingOutput & doramcap.console.v1.RecordingOutput.$Shape;

                /**
                 * Creates a RecordingOutput message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns RecordingOutput
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.RecordingOutput;

                /**
                 * Creates a plain object from a RecordingOutput message. Also converts values to other types if specified.
                 * @param message RecordingOutput
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.RecordingOutput, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this RecordingOutput to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for RecordingOutput
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace RecordingOutput {

                /** Properties of a RecordingOutput. */
                interface $Properties {

                    /** RecordingOutput id */
                    id?: (string|null);

                    /** RecordingOutput path */
                    path?: (string|null);

                    /** RecordingOutput stream */
                    stream?: (string|null);

                    /** RecordingOutput cpu */
                    cpu?: (boolean|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a RecordingOutput. */
                type $Shape = doramcap.console.v1.RecordingOutput.$Properties;
            }

            /**
             * Properties of a ReleaseManifest.
             * @deprecated Use doramcap.console.v1.ReleaseManifest.$Properties instead.
             */
            interface IReleaseManifest extends doramcap.console.v1.ReleaseManifest.$Properties {
            }

            /** Represents a ReleaseManifest. */
            class ReleaseManifest {

                /**
                 * Constructs a new ReleaseManifest.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.console.v1.ReleaseManifest.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ReleaseManifest version. */
                version: string;

                /** ReleaseManifest platform. */
                platform: string;

                /** ReleaseManifest config. */
                config?: (doramcap.console.v1.ConsoleConfig|null);

                /**
                 * Creates a new ReleaseManifest instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ReleaseManifest instance
                 */
                static create(properties: doramcap.console.v1.ReleaseManifest.$Shape): doramcap.console.v1.ReleaseManifest & doramcap.console.v1.ReleaseManifest.$Shape;
                static create(properties?: doramcap.console.v1.ReleaseManifest.$Properties): doramcap.console.v1.ReleaseManifest;

                /**
                 * Encodes the specified ReleaseManifest message. Does not implicitly {@link doramcap.console.v1.ReleaseManifest.verify|verify} messages.
                 * @param message ReleaseManifest message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.console.v1.ReleaseManifest, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ReleaseManifest message, length delimited. Does not implicitly {@link doramcap.console.v1.ReleaseManifest.verify|verify} messages.
                 * @param message ReleaseManifest message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.console.v1.ReleaseManifest, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ReleaseManifest message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.console.v1.ReleaseManifest & doramcap.console.v1.ReleaseManifest.$Shape} ReleaseManifest
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.v1.ReleaseManifest & doramcap.console.v1.ReleaseManifest.$Shape;

                /**
                 * Decodes a ReleaseManifest message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.console.v1.ReleaseManifest & doramcap.console.v1.ReleaseManifest.$Shape} ReleaseManifest
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.v1.ReleaseManifest & doramcap.console.v1.ReleaseManifest.$Shape;

                /**
                 * Creates a ReleaseManifest message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ReleaseManifest
                 */
                static fromObject(object: { [k: string]: any }): doramcap.console.v1.ReleaseManifest;

                /**
                 * Creates a plain object from a ReleaseManifest message. Also converts values to other types if specified.
                 * @param message ReleaseManifest
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.console.v1.ReleaseManifest, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ReleaseManifest to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ReleaseManifest
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ReleaseManifest {

                /** Properties of a ReleaseManifest. */
                interface $Properties {

                    /** ReleaseManifest version */
                    version?: (string|null);

                    /** ReleaseManifest platform */
                    platform?: (string|null);

                    /** ReleaseManifest config */
                    config?: (doramcap.console.v1.ConsoleConfig|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a ReleaseManifest. */
                type $Shape = doramcap.console.v1.ReleaseManifest.$Properties;
            }
        }

        /** Namespace live. */
        namespace live {

            /** Namespace v1. */
            namespace v1 {

                /** Kind enum. */
                enum Kind {

                    /** KIND_UNSPECIFIED value */
                    KIND_UNSPECIFIED = 0,

                    /** CPU_ACCOUNTING value */
                    CPU_ACCOUNTING = 1,

                    /** FAULT_PUBLICATION value */
                    FAULT_PUBLICATION = 2
                }

                /**
                 * Properties of a Frame.
                 * @deprecated Use doramcap.console.live.v1.Frame.$Properties instead.
                 */
                interface IFrame extends doramcap.console.live.v1.Frame.$Properties {
                }

                /** Represents a Frame. */
                class Frame {

                    /**
                     * Constructs a new Frame.
                     * @param [properties] Properties to set
                     */
                    constructor(properties?: doramcap.console.live.v1.Frame.$Properties);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];

                    /** Frame stream. */
                    stream: string;

                    /** Frame sequence. */
                    sequence: (number|Long);

                    /** Frame receivedUnixNs. */
                    receivedUnixNs: (number|Long);

                    /** Frame payload. */
                    payload: Uint8Array;

                    /**
                     * Creates a new Frame instance using the specified properties.
                     * @param [properties] Properties to set
                     * @returns Frame instance
                     */
                    static create(properties: doramcap.console.live.v1.Frame.$Shape): doramcap.console.live.v1.Frame & doramcap.console.live.v1.Frame.$Shape;
                    static create(properties?: doramcap.console.live.v1.Frame.$Properties): doramcap.console.live.v1.Frame;

                    /**
                     * Encodes the specified Frame message. Does not implicitly {@link doramcap.console.live.v1.Frame.verify|verify} messages.
                     * @param message Frame message or plain object to encode
                     * @param [writer] Writer to encode to
                     * @returns Writer
                     */
                    static encode(message: doramcap.console.live.v1.Frame, writer?: $protobuf.Writer): $protobuf.Writer;

                    /**
                     * Encodes the specified Frame message, length delimited. Does not implicitly {@link doramcap.console.live.v1.Frame.verify|verify} messages.
                     * @param message Frame message or plain object to encode
                     * @param [writer] Writer to encode to
                     * @returns Writer
                     */
                    static encodeDelimited(message: doramcap.console.live.v1.Frame, writer?: $protobuf.Writer): $protobuf.Writer;

                    /**
                     * Decodes a Frame message from the specified reader or buffer.
                     * @param reader Reader or buffer to decode from
                     * @param [length] Message length if known beforehand
                     * @returns {doramcap.console.live.v1.Frame & doramcap.console.live.v1.Frame.$Shape} Frame
                     * @throws {Error} If the payload is not a reader or valid buffer
                     * @throws {$protobuf.util.ProtocolError} If required fields are missing
                     */
                    static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.console.live.v1.Frame & doramcap.console.live.v1.Frame.$Shape;

                    /**
                     * Decodes a Frame message from the specified reader or buffer, length delimited.
                     * @param reader Reader or buffer to decode from
                     * @returns {doramcap.console.live.v1.Frame & doramcap.console.live.v1.Frame.$Shape} Frame
                     * @throws {Error} If the payload is not a reader or valid buffer
                     * @throws {$protobuf.util.ProtocolError} If required fields are missing
                     */
                    static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.console.live.v1.Frame & doramcap.console.live.v1.Frame.$Shape;

                    /**
                     * Creates a Frame message from a plain object. Also converts values to their respective internal types.
                     * @param object Plain object
                     * @returns Frame
                     */
                    static fromObject(object: { [k: string]: any }): doramcap.console.live.v1.Frame;

                    /**
                     * Creates a plain object from a Frame message. Also converts values to other types if specified.
                     * @param message Frame
                     * @param [options] Conversion options
                     * @returns Plain object
                     */
                    static toObject(message: doramcap.console.live.v1.Frame, options?: $protobuf.IConversionOptions): { [k: string]: any };

                    /**
                     * Converts this Frame to JSON.
                     * @returns JSON object
                     */
                    toJSON(): { [k: string]: any };

                    /**
                     * Gets the type url for Frame
                     * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                     * @returns The type url
                     */
                    static getTypeUrl(prefix?: string): string;
                }

                namespace Frame {

                    /** Properties of a Frame. */
                    interface $Properties {

                        /** Frame stream */
                        stream?: (string|null);

                        /** Frame sequence */
                        sequence?: (number|Long|null);

                        /** Frame receivedUnixNs */
                        receivedUnixNs?: (number|Long|null);

                        /** Frame payload */
                        payload?: (Uint8Array|null);

                        /** Unknown fields preserved while decoding when enabled */
                        $unknowns?: Uint8Array[];
                    }

                    /** Shape of a Frame. */
                    type $Shape = doramcap.console.live.v1.Frame.$Properties;
                }
            }
        }
    }

    /** Namespace compose. */
    namespace compose {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of a ComposeProject.
             * @deprecated Use doramcap.compose.v1.ComposeProject.$Properties instead.
             */
            interface IComposeProject extends doramcap.compose.v1.ComposeProject.$Properties {
            }

            /** Represents a ComposeProject. */
            class ComposeProject {

                /**
                 * Constructs a new ComposeProject.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.ComposeProject.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ComposeProject key. */
                key: string;

                /** ComposeProject directory. */
                directory: string;

                /** ComposeProject files. */
                files: string[];

                /** ComposeProject envFiles. */
                envFiles: string[];

                /** ComposeProject environment. */
                environment: { [k: string]: string };

                /** ComposeProject profiles. */
                profiles: string[];

                /** ComposeProject services. */
                services: string[];

                /** ComposeProject completedServices. */
                completedServices: string[];

                /** ComposeProject removeVolumes. */
                removeVolumes: boolean;

                /**
                 * Creates a new ComposeProject instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ComposeProject instance
                 */
                static create(properties: doramcap.compose.v1.ComposeProject.$Shape): doramcap.compose.v1.ComposeProject & doramcap.compose.v1.ComposeProject.$Shape;
                static create(properties?: doramcap.compose.v1.ComposeProject.$Properties): doramcap.compose.v1.ComposeProject;

                /**
                 * Encodes the specified ComposeProject message. Does not implicitly {@link doramcap.compose.v1.ComposeProject.verify|verify} messages.
                 * @param message ComposeProject message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.ComposeProject, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ComposeProject message, length delimited. Does not implicitly {@link doramcap.compose.v1.ComposeProject.verify|verify} messages.
                 * @param message ComposeProject message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.ComposeProject, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ComposeProject message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.ComposeProject & doramcap.compose.v1.ComposeProject.$Shape} ComposeProject
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.ComposeProject & doramcap.compose.v1.ComposeProject.$Shape;

                /**
                 * Decodes a ComposeProject message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.ComposeProject & doramcap.compose.v1.ComposeProject.$Shape} ComposeProject
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.ComposeProject & doramcap.compose.v1.ComposeProject.$Shape;

                /**
                 * Creates a ComposeProject message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ComposeProject
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.ComposeProject;

                /**
                 * Creates a plain object from a ComposeProject message. Also converts values to other types if specified.
                 * @param message ComposeProject
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.ComposeProject, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ComposeProject to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ComposeProject
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ComposeProject {

                /** Properties of a ComposeProject. */
                interface $Properties {

                    /** ComposeProject key */
                    key?: (string|null);

                    /** ComposeProject directory */
                    directory?: (string|null);

                    /** ComposeProject files */
                    files?: (string[]|null);

                    /** ComposeProject envFiles */
                    envFiles?: (string[]|null);

                    /** ComposeProject environment */
                    environment?: ({ [k: string]: string }|null);

                    /** ComposeProject profiles */
                    profiles?: (string[]|null);

                    /** ComposeProject services */
                    services?: (string[]|null);

                    /** ComposeProject completedServices */
                    completedServices?: (string[]|null);

                    /** ComposeProject removeVolumes */
                    removeVolumes?: (boolean|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a ComposeProject. */
                type $Shape = doramcap.compose.v1.ComposeProject.$Properties;
            }

            /**
             * Properties of a ResolvedComposeProject.
             * @deprecated Use doramcap.compose.v1.ResolvedComposeProject.$Properties instead.
             */
            interface IResolvedComposeProject extends doramcap.compose.v1.ResolvedComposeProject.$Properties {
            }

            /** Represents a ResolvedComposeProject. */
            class ResolvedComposeProject {

                /**
                 * Constructs a new ResolvedComposeProject.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.ResolvedComposeProject.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ResolvedComposeProject run. */
                run: string;

                /** ResolvedComposeProject key. */
                key: string;

                /** ResolvedComposeProject name. */
                name: string;

                /** ResolvedComposeProject manifest. */
                manifest: string;

                /** ResolvedComposeProject sha256. */
                sha256: string;

                /** ResolvedComposeProject services. */
                services: { [k: string]: doramcap.compose.v1.ComposeService };

                /** ResolvedComposeProject diagnostics. */
                diagnostics: string[];

                /** ResolvedComposeProject removeVolumes. */
                removeVolumes: boolean;

                /**
                 * Creates a new ResolvedComposeProject instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ResolvedComposeProject instance
                 */
                static create(properties: doramcap.compose.v1.ResolvedComposeProject.$Shape): doramcap.compose.v1.ResolvedComposeProject & doramcap.compose.v1.ResolvedComposeProject.$Shape;
                static create(properties?: doramcap.compose.v1.ResolvedComposeProject.$Properties): doramcap.compose.v1.ResolvedComposeProject;

                /**
                 * Encodes the specified ResolvedComposeProject message. Does not implicitly {@link doramcap.compose.v1.ResolvedComposeProject.verify|verify} messages.
                 * @param message ResolvedComposeProject message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.ResolvedComposeProject, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ResolvedComposeProject message, length delimited. Does not implicitly {@link doramcap.compose.v1.ResolvedComposeProject.verify|verify} messages.
                 * @param message ResolvedComposeProject message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.ResolvedComposeProject, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ResolvedComposeProject message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.ResolvedComposeProject & doramcap.compose.v1.ResolvedComposeProject.$Shape} ResolvedComposeProject
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.ResolvedComposeProject & doramcap.compose.v1.ResolvedComposeProject.$Shape;

                /**
                 * Decodes a ResolvedComposeProject message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.ResolvedComposeProject & doramcap.compose.v1.ResolvedComposeProject.$Shape} ResolvedComposeProject
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.ResolvedComposeProject & doramcap.compose.v1.ResolvedComposeProject.$Shape;

                /**
                 * Creates a ResolvedComposeProject message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ResolvedComposeProject
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.ResolvedComposeProject;

                /**
                 * Creates a plain object from a ResolvedComposeProject message. Also converts values to other types if specified.
                 * @param message ResolvedComposeProject
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.ResolvedComposeProject, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ResolvedComposeProject to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ResolvedComposeProject
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ResolvedComposeProject {

                /** Properties of a ResolvedComposeProject. */
                interface $Properties {

                    /** ResolvedComposeProject run */
                    run?: (string|null);

                    /** ResolvedComposeProject key */
                    key?: (string|null);

                    /** ResolvedComposeProject name */
                    name?: (string|null);

                    /** ResolvedComposeProject manifest */
                    manifest?: (string|null);

                    /** ResolvedComposeProject sha256 */
                    sha256?: (string|null);

                    /** ResolvedComposeProject services */
                    services?: ({ [k: string]: doramcap.compose.v1.ComposeService }|null);

                    /** ResolvedComposeProject diagnostics */
                    diagnostics?: (string[]|null);

                    /** ResolvedComposeProject removeVolumes */
                    removeVolumes?: (boolean|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a ResolvedComposeProject. */
                type $Shape = doramcap.compose.v1.ResolvedComposeProject.$Properties;
            }

            /**
             * Properties of a ComposeService.
             * @deprecated Use doramcap.compose.v1.ComposeService.$Properties instead.
             */
            interface IComposeService extends doramcap.compose.v1.ComposeService.$Properties {
            }

            /** Represents a ComposeService. */
            class ComposeService {

                /**
                 * Constructs a new ComposeService.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.ComposeService.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ComposeService imageId. */
                imageId: string;

                /** ComposeService replicas. */
                replicas: (number|Long);

                /** ComposeService completed. */
                completed: boolean;

                /**
                 * Creates a new ComposeService instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ComposeService instance
                 */
                static create(properties: doramcap.compose.v1.ComposeService.$Shape): doramcap.compose.v1.ComposeService & doramcap.compose.v1.ComposeService.$Shape;
                static create(properties?: doramcap.compose.v1.ComposeService.$Properties): doramcap.compose.v1.ComposeService;

                /**
                 * Encodes the specified ComposeService message. Does not implicitly {@link doramcap.compose.v1.ComposeService.verify|verify} messages.
                 * @param message ComposeService message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.ComposeService, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ComposeService message, length delimited. Does not implicitly {@link doramcap.compose.v1.ComposeService.verify|verify} messages.
                 * @param message ComposeService message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.ComposeService, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ComposeService message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.ComposeService & doramcap.compose.v1.ComposeService.$Shape} ComposeService
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.ComposeService & doramcap.compose.v1.ComposeService.$Shape;

                /**
                 * Decodes a ComposeService message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.ComposeService & doramcap.compose.v1.ComposeService.$Shape} ComposeService
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.ComposeService & doramcap.compose.v1.ComposeService.$Shape;

                /**
                 * Creates a ComposeService message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ComposeService
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.ComposeService;

                /**
                 * Creates a plain object from a ComposeService message. Also converts values to other types if specified.
                 * @param message ComposeService
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.ComposeService, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ComposeService to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ComposeService
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ComposeService {

                /** Properties of a ComposeService. */
                interface $Properties {

                    /** ComposeService imageId */
                    imageId?: (string|null);

                    /** ComposeService replicas */
                    replicas?: (number|Long|null);

                    /** ComposeService completed */
                    completed?: (boolean|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a ComposeService. */
                type $Shape = doramcap.compose.v1.ComposeService.$Properties;
            }

            /**
             * Properties of a ResolveInput.
             * @deprecated Use doramcap.compose.v1.ResolveInput.$Properties instead.
             */
            interface IResolveInput extends doramcap.compose.v1.ResolveInput.$Properties {
            }

            /** Represents a ResolveInput. */
            class ResolveInput {

                /**
                 * Constructs a new ResolveInput.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.ResolveInput.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ResolveInput run. */
                run: string;

                /** ResolveInput artifacts. */
                artifacts: string;

                /** ResolveInput project. */
                project?: (doramcap.compose.v1.ComposeProject|null);

                /** ResolveInput repositoryRoot. */
                repositoryRoot?: (string|null);

                /**
                 * Creates a new ResolveInput instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ResolveInput instance
                 */
                static create(properties: doramcap.compose.v1.ResolveInput.$Shape): doramcap.compose.v1.ResolveInput & doramcap.compose.v1.ResolveInput.$Shape;
                static create(properties?: doramcap.compose.v1.ResolveInput.$Properties): doramcap.compose.v1.ResolveInput;

                /**
                 * Encodes the specified ResolveInput message. Does not implicitly {@link doramcap.compose.v1.ResolveInput.verify|verify} messages.
                 * @param message ResolveInput message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.ResolveInput, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ResolveInput message, length delimited. Does not implicitly {@link doramcap.compose.v1.ResolveInput.verify|verify} messages.
                 * @param message ResolveInput message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.ResolveInput, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ResolveInput message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.ResolveInput & doramcap.compose.v1.ResolveInput.$Shape} ResolveInput
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.ResolveInput & doramcap.compose.v1.ResolveInput.$Shape;

                /**
                 * Decodes a ResolveInput message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.ResolveInput & doramcap.compose.v1.ResolveInput.$Shape} ResolveInput
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.ResolveInput & doramcap.compose.v1.ResolveInput.$Shape;

                /**
                 * Creates a ResolveInput message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ResolveInput
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.ResolveInput;

                /**
                 * Creates a plain object from a ResolveInput message. Also converts values to other types if specified.
                 * @param message ResolveInput
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.ResolveInput, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ResolveInput to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ResolveInput
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ResolveInput {

                /** Properties of a ResolveInput. */
                interface $Properties {

                    /** ResolveInput run */
                    run?: (string|null);

                    /** ResolveInput artifacts */
                    artifacts?: (string|null);

                    /** ResolveInput project */
                    project?: (doramcap.compose.v1.ComposeProject|null);

                    /** ResolveInput repositoryRoot */
                    repositoryRoot?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a ResolveInput. */
                type $Shape = doramcap.compose.v1.ResolveInput.$Properties;
            }

            /**
             * Properties of a StartInput.
             * @deprecated Use doramcap.compose.v1.StartInput.$Properties instead.
             */
            interface IStartInput extends doramcap.compose.v1.StartInput.$Properties {
            }

            /** Represents a StartInput. */
            class StartInput {

                /**
                 * Constructs a new StartInput.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.StartInput.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** StartInput services. */
                services: string[];

                /** StartInput plan. */
                plan?: (doramcap.compose.v1.ResolvedComposeProject|null);

                /** StartInput readinessSeconds. */
                readinessSeconds: (number|Long);

                /**
                 * Creates a new StartInput instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns StartInput instance
                 */
                static create(properties: doramcap.compose.v1.StartInput.$Shape): doramcap.compose.v1.StartInput & doramcap.compose.v1.StartInput.$Shape;
                static create(properties?: doramcap.compose.v1.StartInput.$Properties): doramcap.compose.v1.StartInput;

                /**
                 * Encodes the specified StartInput message. Does not implicitly {@link doramcap.compose.v1.StartInput.verify|verify} messages.
                 * @param message StartInput message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.StartInput, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified StartInput message, length delimited. Does not implicitly {@link doramcap.compose.v1.StartInput.verify|verify} messages.
                 * @param message StartInput message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.StartInput, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a StartInput message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.StartInput & doramcap.compose.v1.StartInput.$Shape} StartInput
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.StartInput & doramcap.compose.v1.StartInput.$Shape;

                /**
                 * Decodes a StartInput message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.StartInput & doramcap.compose.v1.StartInput.$Shape} StartInput
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.StartInput & doramcap.compose.v1.StartInput.$Shape;

                /**
                 * Creates a StartInput message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns StartInput
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.StartInput;

                /**
                 * Creates a plain object from a StartInput message. Also converts values to other types if specified.
                 * @param message StartInput
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.StartInput, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this StartInput to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for StartInput
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace StartInput {

                /** Properties of a StartInput. */
                interface $Properties {

                    /** StartInput services */
                    services?: (string[]|null);

                    /** StartInput plan */
                    plan?: (doramcap.compose.v1.ResolvedComposeProject|null);

                    /** StartInput readinessSeconds */
                    readinessSeconds?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a StartInput. */
                type $Shape = doramcap.compose.v1.StartInput.$Properties;
            }

            /**
             * Properties of a StopInput.
             * @deprecated Use doramcap.compose.v1.StopInput.$Properties instead.
             */
            interface IStopInput extends doramcap.compose.v1.StopInput.$Properties {
            }

            /** Represents a StopInput. */
            class StopInput {

                /**
                 * Constructs a new StopInput.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.StopInput.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** StopInput allowVolumeDeletion. */
                allowVolumeDeletion: boolean;

                /** StopInput services. */
                services: string[];

                /** StopInput plan. */
                plan?: (doramcap.compose.v1.ResolvedComposeProject|null);

                /** StopInput stopSeconds. */
                stopSeconds: (number|Long);

                /**
                 * Creates a new StopInput instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns StopInput instance
                 */
                static create(properties: doramcap.compose.v1.StopInput.$Shape): doramcap.compose.v1.StopInput & doramcap.compose.v1.StopInput.$Shape;
                static create(properties?: doramcap.compose.v1.StopInput.$Properties): doramcap.compose.v1.StopInput;

                /**
                 * Encodes the specified StopInput message. Does not implicitly {@link doramcap.compose.v1.StopInput.verify|verify} messages.
                 * @param message StopInput message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.StopInput, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified StopInput message, length delimited. Does not implicitly {@link doramcap.compose.v1.StopInput.verify|verify} messages.
                 * @param message StopInput message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.StopInput, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a StopInput message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.StopInput & doramcap.compose.v1.StopInput.$Shape} StopInput
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.StopInput & doramcap.compose.v1.StopInput.$Shape;

                /**
                 * Decodes a StopInput message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.StopInput & doramcap.compose.v1.StopInput.$Shape} StopInput
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.StopInput & doramcap.compose.v1.StopInput.$Shape;

                /**
                 * Creates a StopInput message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns StopInput
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.StopInput;

                /**
                 * Creates a plain object from a StopInput message. Also converts values to other types if specified.
                 * @param message StopInput
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.StopInput, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this StopInput to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for StopInput
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace StopInput {

                /** Properties of a StopInput. */
                interface $Properties {

                    /** StopInput allowVolumeDeletion */
                    allowVolumeDeletion?: (boolean|null);

                    /** StopInput services */
                    services?: (string[]|null);

                    /** StopInput plan */
                    plan?: (doramcap.compose.v1.ResolvedComposeProject|null);

                    /** StopInput stopSeconds */
                    stopSeconds?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a StopInput. */
                type $Shape = doramcap.compose.v1.StopInput.$Properties;
            }

            /**
             * Properties of a Diagnostic.
             * @deprecated Use doramcap.compose.v1.Diagnostic.$Properties instead.
             */
            interface IDiagnostic extends doramcap.compose.v1.Diagnostic.$Properties {
            }

            /** Represents a Diagnostic. */
            class Diagnostic {

                /**
                 * Constructs a new Diagnostic.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.Diagnostic.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Diagnostic operation. */
                operation: string;

                /** Diagnostic kind. */
                kind: string;

                /** Diagnostic message. */
                message: string;

                /** Diagnostic status. */
                status?: (number|Long|null);

                /** Diagnostic stdout. */
                stdout?: (string|null);

                /** Diagnostic stderr. */
                stderr?: (string|null);

                /**
                 * Creates a new Diagnostic instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Diagnostic instance
                 */
                static create(properties: doramcap.compose.v1.Diagnostic.$Shape): doramcap.compose.v1.Diagnostic & doramcap.compose.v1.Diagnostic.$Shape;
                static create(properties?: doramcap.compose.v1.Diagnostic.$Properties): doramcap.compose.v1.Diagnostic;

                /**
                 * Encodes the specified Diagnostic message. Does not implicitly {@link doramcap.compose.v1.Diagnostic.verify|verify} messages.
                 * @param message Diagnostic message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.Diagnostic, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Diagnostic message, length delimited. Does not implicitly {@link doramcap.compose.v1.Diagnostic.verify|verify} messages.
                 * @param message Diagnostic message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.Diagnostic, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Diagnostic message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.Diagnostic & doramcap.compose.v1.Diagnostic.$Shape} Diagnostic
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.Diagnostic & doramcap.compose.v1.Diagnostic.$Shape;

                /**
                 * Decodes a Diagnostic message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.Diagnostic & doramcap.compose.v1.Diagnostic.$Shape} Diagnostic
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.Diagnostic & doramcap.compose.v1.Diagnostic.$Shape;

                /**
                 * Creates a Diagnostic message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Diagnostic
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.Diagnostic;

                /**
                 * Creates a plain object from a Diagnostic message. Also converts values to other types if specified.
                 * @param message Diagnostic
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.Diagnostic, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Diagnostic to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Diagnostic
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Diagnostic {

                /** Properties of a Diagnostic. */
                interface $Properties {

                    /** Diagnostic operation */
                    operation?: (string|null);

                    /** Diagnostic kind */
                    kind?: (string|null);

                    /** Diagnostic message */
                    message?: (string|null);

                    /** Diagnostic status */
                    status?: (number|Long|null);

                    /** Diagnostic stdout */
                    stdout?: (string|null);

                    /** Diagnostic stderr */
                    stderr?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Diagnostic. */
                type $Shape = doramcap.compose.v1.Diagnostic.$Properties;
            }

            /**
             * Properties of a Container.
             * @deprecated Use doramcap.compose.v1.Container.$Properties instead.
             */
            interface IContainer extends doramcap.compose.v1.Container.$Properties {
            }

            /** Represents a Container. */
            class Container {

                /**
                 * Constructs a new Container.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.Container.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Container id. */
                id: string;

                /** Container service. */
                service: string;

                /** Container imageId. */
                imageId: string;

                /** Container pid. */
                pid: (number|Long);

                /** Container status. */
                status: string;

                /** Container health. */
                health?: (string|null);

                /** Container exitCode. */
                exitCode?: (number|Long|null);

                /** Container restartCount. */
                restartCount: (number|Long);

                /** Container lastOutput. */
                lastOutput: string;

                /**
                 * Creates a new Container instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Container instance
                 */
                static create(properties: doramcap.compose.v1.Container.$Shape): doramcap.compose.v1.Container & doramcap.compose.v1.Container.$Shape;
                static create(properties?: doramcap.compose.v1.Container.$Properties): doramcap.compose.v1.Container;

                /**
                 * Encodes the specified Container message. Does not implicitly {@link doramcap.compose.v1.Container.verify|verify} messages.
                 * @param message Container message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.Container, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Container message, length delimited. Does not implicitly {@link doramcap.compose.v1.Container.verify|verify} messages.
                 * @param message Container message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.Container, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Container message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.Container & doramcap.compose.v1.Container.$Shape} Container
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.Container & doramcap.compose.v1.Container.$Shape;

                /**
                 * Decodes a Container message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.Container & doramcap.compose.v1.Container.$Shape} Container
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.Container & doramcap.compose.v1.Container.$Shape;

                /**
                 * Creates a Container message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Container
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.Container;

                /**
                 * Creates a plain object from a Container message. Also converts values to other types if specified.
                 * @param message Container
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.Container, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Container to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Container
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Container {

                /** Properties of a Container. */
                interface $Properties {

                    /** Container id */
                    id?: (string|null);

                    /** Container service */
                    service?: (string|null);

                    /** Container imageId */
                    imageId?: (string|null);

                    /** Container pid */
                    pid?: (number|Long|null);

                    /** Container status */
                    status?: (string|null);

                    /** Container health */
                    health?: (string|null);

                    /** Container exitCode */
                    exitCode?: (number|Long|null);

                    /** Container restartCount */
                    restartCount?: (number|Long|null);

                    /** Container lastOutput */
                    lastOutput?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Container. */
                type $Shape = doramcap.compose.v1.Container.$Properties;
            }

            /**
             * Properties of a ComposeProjectState.
             * @deprecated Use doramcap.compose.v1.ComposeProjectState.$Properties instead.
             */
            interface IComposeProjectState extends doramcap.compose.v1.ComposeProjectState.$Properties {
            }

            /** Represents a ComposeProjectState. */
            class ComposeProjectState {

                /**
                 * Constructs a new ComposeProjectState.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.ComposeProjectState.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ComposeProjectState plan. */
                plan?: (doramcap.compose.v1.ResolvedComposeProject|null);

                /** ComposeProjectState containers. */
                containers: doramcap.compose.v1.Container[];

                /**
                 * Creates a new ComposeProjectState instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ComposeProjectState instance
                 */
                static create(properties: doramcap.compose.v1.ComposeProjectState.$Shape): doramcap.compose.v1.ComposeProjectState & doramcap.compose.v1.ComposeProjectState.$Shape;
                static create(properties?: doramcap.compose.v1.ComposeProjectState.$Properties): doramcap.compose.v1.ComposeProjectState;

                /**
                 * Encodes the specified ComposeProjectState message. Does not implicitly {@link doramcap.compose.v1.ComposeProjectState.verify|verify} messages.
                 * @param message ComposeProjectState message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.ComposeProjectState, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ComposeProjectState message, length delimited. Does not implicitly {@link doramcap.compose.v1.ComposeProjectState.verify|verify} messages.
                 * @param message ComposeProjectState message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.ComposeProjectState, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ComposeProjectState message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.ComposeProjectState & doramcap.compose.v1.ComposeProjectState.$Shape} ComposeProjectState
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.ComposeProjectState & doramcap.compose.v1.ComposeProjectState.$Shape;

                /**
                 * Decodes a ComposeProjectState message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.ComposeProjectState & doramcap.compose.v1.ComposeProjectState.$Shape} ComposeProjectState
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.ComposeProjectState & doramcap.compose.v1.ComposeProjectState.$Shape;

                /**
                 * Creates a ComposeProjectState message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ComposeProjectState
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.ComposeProjectState;

                /**
                 * Creates a plain object from a ComposeProjectState message. Also converts values to other types if specified.
                 * @param message ComposeProjectState
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.ComposeProjectState, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ComposeProjectState to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ComposeProjectState
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ComposeProjectState {

                /** Properties of a ComposeProjectState. */
                interface $Properties {

                    /** ComposeProjectState plan */
                    plan?: (doramcap.compose.v1.ResolvedComposeProject|null);

                    /** ComposeProjectState containers */
                    containers?: (doramcap.compose.v1.Container[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a ComposeProjectState. */
                type $Shape = doramcap.compose.v1.ComposeProjectState.$Properties;
            }

            /**
             * Properties of a Cleanup.
             * @deprecated Use doramcap.compose.v1.Cleanup.$Properties instead.
             */
            interface ICleanup extends doramcap.compose.v1.Cleanup.$Properties {
            }

            /** Represents a Cleanup. */
            class Cleanup {

                /**
                 * Constructs a new Cleanup.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.Cleanup.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Cleanup project. */
                project: string;

                /** Cleanup diagnostics. */
                diagnostics: doramcap.compose.v1.Diagnostic[];

                /** Cleanup remaining. */
                remaining: string[];

                /** Cleanup retainedVolumes. */
                retainedVolumes: string[];

                /** Cleanup verified. */
                verified: boolean;

                /**
                 * Creates a new Cleanup instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Cleanup instance
                 */
                static create(properties: doramcap.compose.v1.Cleanup.$Shape): doramcap.compose.v1.Cleanup & doramcap.compose.v1.Cleanup.$Shape;
                static create(properties?: doramcap.compose.v1.Cleanup.$Properties): doramcap.compose.v1.Cleanup;

                /**
                 * Encodes the specified Cleanup message. Does not implicitly {@link doramcap.compose.v1.Cleanup.verify|verify} messages.
                 * @param message Cleanup message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.Cleanup, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Cleanup message, length delimited. Does not implicitly {@link doramcap.compose.v1.Cleanup.verify|verify} messages.
                 * @param message Cleanup message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.Cleanup, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Cleanup message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.Cleanup & doramcap.compose.v1.Cleanup.$Shape} Cleanup
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.Cleanup & doramcap.compose.v1.Cleanup.$Shape;

                /**
                 * Decodes a Cleanup message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.Cleanup & doramcap.compose.v1.Cleanup.$Shape} Cleanup
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.Cleanup & doramcap.compose.v1.Cleanup.$Shape;

                /**
                 * Creates a Cleanup message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Cleanup
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.Cleanup;

                /**
                 * Creates a plain object from a Cleanup message. Also converts values to other types if specified.
                 * @param message Cleanup
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.Cleanup, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Cleanup to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Cleanup
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Cleanup {

                /** Properties of a Cleanup. */
                interface $Properties {

                    /** Cleanup project */
                    project?: (string|null);

                    /** Cleanup diagnostics */
                    diagnostics?: (doramcap.compose.v1.Diagnostic[]|null);

                    /** Cleanup remaining */
                    remaining?: (string[]|null);

                    /** Cleanup retainedVolumes */
                    retainedVolumes?: (string[]|null);

                    /** Cleanup verified */
                    verified?: (boolean|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Cleanup. */
                type $Shape = doramcap.compose.v1.Cleanup.$Properties;
            }

            /**
             * Properties of a Shutdown.
             * @deprecated Use doramcap.compose.v1.Shutdown.$Properties instead.
             */
            interface IShutdown extends doramcap.compose.v1.Shutdown.$Properties {
            }

            /** Represents a Shutdown. */
            class Shutdown {

                /**
                 * Constructs a new Shutdown.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.Shutdown.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Shutdown project. */
                project: string;

                /** Shutdown diagnostics. */
                diagnostics: doramcap.compose.v1.Diagnostic[];

                /** Shutdown stopped. */
                stopped: boolean;

                /**
                 * Creates a new Shutdown instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Shutdown instance
                 */
                static create(properties: doramcap.compose.v1.Shutdown.$Shape): doramcap.compose.v1.Shutdown & doramcap.compose.v1.Shutdown.$Shape;
                static create(properties?: doramcap.compose.v1.Shutdown.$Properties): doramcap.compose.v1.Shutdown;

                /**
                 * Encodes the specified Shutdown message. Does not implicitly {@link doramcap.compose.v1.Shutdown.verify|verify} messages.
                 * @param message Shutdown message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.Shutdown, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Shutdown message, length delimited. Does not implicitly {@link doramcap.compose.v1.Shutdown.verify|verify} messages.
                 * @param message Shutdown message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.Shutdown, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Shutdown message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.Shutdown & doramcap.compose.v1.Shutdown.$Shape} Shutdown
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.Shutdown & doramcap.compose.v1.Shutdown.$Shape;

                /**
                 * Decodes a Shutdown message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.Shutdown & doramcap.compose.v1.Shutdown.$Shape} Shutdown
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.Shutdown & doramcap.compose.v1.Shutdown.$Shape;

                /**
                 * Creates a Shutdown message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Shutdown
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.Shutdown;

                /**
                 * Creates a plain object from a Shutdown message. Also converts values to other types if specified.
                 * @param message Shutdown
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.Shutdown, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Shutdown to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Shutdown
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Shutdown {

                /** Properties of a Shutdown. */
                interface $Properties {

                    /** Shutdown project */
                    project?: (string|null);

                    /** Shutdown diagnostics */
                    diagnostics?: (doramcap.compose.v1.Diagnostic[]|null);

                    /** Shutdown stopped */
                    stopped?: (boolean|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Shutdown. */
                type $Shape = doramcap.compose.v1.Shutdown.$Properties;
            }

            /**
             * Properties of a ConfigurationSnapshot.
             * @deprecated Use doramcap.compose.v1.ConfigurationSnapshot.$Properties instead.
             */
            interface IConfigurationSnapshot extends doramcap.compose.v1.ConfigurationSnapshot.$Properties {
            }

            /** Represents a ConfigurationSnapshot. */
            class ConfigurationSnapshot {

                /**
                 * Constructs a new ConfigurationSnapshot.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.compose.v1.ConfigurationSnapshot.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ConfigurationSnapshot manifestSha256. */
                manifestSha256: string;

                /** ConfigurationSnapshot systemServices. */
                systemServices: string[];

                /** ConfigurationSnapshot userServices. */
                userServices: string[];

                /** ConfigurationSnapshot settingsSha256. */
                settingsSha256: string;

                /** ConfigurationSnapshot graphSha256. */
                graphSha256: string;

                /** ConfigurationSnapshot services. */
                services: { [k: string]: doramcap.compose.v1.ComposeService };

                /** ConfigurationSnapshot removeVolumes. */
                removeVolumes: boolean;

                /**
                 * Creates a new ConfigurationSnapshot instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ConfigurationSnapshot instance
                 */
                static create(properties: doramcap.compose.v1.ConfigurationSnapshot.$Shape): doramcap.compose.v1.ConfigurationSnapshot & doramcap.compose.v1.ConfigurationSnapshot.$Shape;
                static create(properties?: doramcap.compose.v1.ConfigurationSnapshot.$Properties): doramcap.compose.v1.ConfigurationSnapshot;

                /**
                 * Encodes the specified ConfigurationSnapshot message. Does not implicitly {@link doramcap.compose.v1.ConfigurationSnapshot.verify|verify} messages.
                 * @param message ConfigurationSnapshot message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.compose.v1.ConfigurationSnapshot, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ConfigurationSnapshot message, length delimited. Does not implicitly {@link doramcap.compose.v1.ConfigurationSnapshot.verify|verify} messages.
                 * @param message ConfigurationSnapshot message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.compose.v1.ConfigurationSnapshot, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ConfigurationSnapshot message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.compose.v1.ConfigurationSnapshot & doramcap.compose.v1.ConfigurationSnapshot.$Shape} ConfigurationSnapshot
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.compose.v1.ConfigurationSnapshot & doramcap.compose.v1.ConfigurationSnapshot.$Shape;

                /**
                 * Decodes a ConfigurationSnapshot message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.compose.v1.ConfigurationSnapshot & doramcap.compose.v1.ConfigurationSnapshot.$Shape} ConfigurationSnapshot
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.compose.v1.ConfigurationSnapshot & doramcap.compose.v1.ConfigurationSnapshot.$Shape;

                /**
                 * Creates a ConfigurationSnapshot message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ConfigurationSnapshot
                 */
                static fromObject(object: { [k: string]: any }): doramcap.compose.v1.ConfigurationSnapshot;

                /**
                 * Creates a plain object from a ConfigurationSnapshot message. Also converts values to other types if specified.
                 * @param message ConfigurationSnapshot
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.compose.v1.ConfigurationSnapshot, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ConfigurationSnapshot to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ConfigurationSnapshot
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ConfigurationSnapshot {

                /** Properties of a ConfigurationSnapshot. */
                interface $Properties {

                    /** ConfigurationSnapshot manifestSha256 */
                    manifestSha256?: (string|null);

                    /** ConfigurationSnapshot systemServices */
                    systemServices?: (string[]|null);

                    /** ConfigurationSnapshot userServices */
                    userServices?: (string[]|null);

                    /** ConfigurationSnapshot settingsSha256 */
                    settingsSha256?: (string|null);

                    /** ConfigurationSnapshot graphSha256 */
                    graphSha256?: (string|null);

                    /** ConfigurationSnapshot services */
                    services?: ({ [k: string]: doramcap.compose.v1.ComposeService }|null);

                    /** ConfigurationSnapshot removeVolumes */
                    removeVolumes?: (boolean|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a ConfigurationSnapshot. */
                type $Shape = doramcap.compose.v1.ConfigurationSnapshot.$Properties;
            }
        }
    }

    /** Namespace dora. */
    namespace dora {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of a DoraRuntime.
             * @deprecated Use doramcap.dora.v1.DoraRuntime.$Properties instead.
             */
            interface IDoraRuntime extends doramcap.dora.v1.DoraRuntime.$Properties {
            }

            /** Represents a DoraRuntime. */
            class DoraRuntime {

                /**
                 * Constructs a new DoraRuntime.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.dora.v1.DoraRuntime.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** DoraRuntime image. */
                image: string;

                /** DoraRuntime coordinatorPort. */
                coordinatorPort: number;

                /** DoraRuntime daemonPort. */
                daemonPort: number;

                /** DoraRuntime graph. */
                graph: string;

                /** DoraRuntime runtimeDirectory. */
                runtimeDirectory: string;

                /** DoraRuntime shmBytes. */
                shmBytes: (number|Long);

                /** DoraRuntime memlockBytes. */
                memlockBytes: (number|Long);

                /** DoraRuntime zenohOverlay. */
                zenohOverlay: string;

                /** DoraRuntime participantUid. */
                participantUid?: (number|null);

                /** DoraRuntime participantGid. */
                participantGid?: (number|null);

                /** DoraRuntime graphSha256. */
                graphSha256: string;

                /**
                 * Creates a new DoraRuntime instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns DoraRuntime instance
                 */
                static create(properties: doramcap.dora.v1.DoraRuntime.$Shape): doramcap.dora.v1.DoraRuntime & doramcap.dora.v1.DoraRuntime.$Shape;
                static create(properties?: doramcap.dora.v1.DoraRuntime.$Properties): doramcap.dora.v1.DoraRuntime;

                /**
                 * Encodes the specified DoraRuntime message. Does not implicitly {@link doramcap.dora.v1.DoraRuntime.verify|verify} messages.
                 * @param message DoraRuntime message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.dora.v1.DoraRuntime, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified DoraRuntime message, length delimited. Does not implicitly {@link doramcap.dora.v1.DoraRuntime.verify|verify} messages.
                 * @param message DoraRuntime message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.dora.v1.DoraRuntime, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a DoraRuntime message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.dora.v1.DoraRuntime & doramcap.dora.v1.DoraRuntime.$Shape} DoraRuntime
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.dora.v1.DoraRuntime & doramcap.dora.v1.DoraRuntime.$Shape;

                /**
                 * Decodes a DoraRuntime message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.dora.v1.DoraRuntime & doramcap.dora.v1.DoraRuntime.$Shape} DoraRuntime
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.dora.v1.DoraRuntime & doramcap.dora.v1.DoraRuntime.$Shape;

                /**
                 * Creates a DoraRuntime message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns DoraRuntime
                 */
                static fromObject(object: { [k: string]: any }): doramcap.dora.v1.DoraRuntime;

                /**
                 * Creates a plain object from a DoraRuntime message. Also converts values to other types if specified.
                 * @param message DoraRuntime
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.dora.v1.DoraRuntime, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this DoraRuntime to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for DoraRuntime
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace DoraRuntime {

                /** Properties of a DoraRuntime. */
                interface $Properties {

                    /** DoraRuntime image */
                    image?: (string|null);

                    /** DoraRuntime coordinatorPort */
                    coordinatorPort?: (number|null);

                    /** DoraRuntime daemonPort */
                    daemonPort?: (number|null);

                    /** DoraRuntime graph */
                    graph?: (string|null);

                    /** DoraRuntime runtimeDirectory */
                    runtimeDirectory?: (string|null);

                    /** DoraRuntime shmBytes */
                    shmBytes?: (number|Long|null);

                    /** DoraRuntime memlockBytes */
                    memlockBytes?: (number|Long|null);

                    /** DoraRuntime zenohOverlay */
                    zenohOverlay?: (string|null);

                    /** DoraRuntime participantUid */
                    participantUid?: (number|null);

                    /** DoraRuntime participantGid */
                    participantGid?: (number|null);

                    /** DoraRuntime graphSha256 */
                    graphSha256?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a DoraRuntime. */
                type $Shape = doramcap.dora.v1.DoraRuntime.$Properties;
            }
        }
    }

    /** Namespace workflow. */
    namespace workflow {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of an Event.
             * @deprecated Use doramcap.workflow.v1.Event.$Properties instead.
             */
            interface IEvent extends doramcap.workflow.v1.Event.$Properties {
            }

            /** Represents an Event. */
            class Event {

                /**
                 * Constructs a new Event.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.workflow.v1.Event.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Event version. */
                version: number;

                /** Event sequence. */
                sequence: (number|Long);

                /** Event session. */
                session: string;

                /** Event invocation. */
                invocation: (number|Long);

                /** Event step. */
                step: string;

                /** Event stepVersion. */
                stepVersion: number;

                /** Event executor. */
                executor: string;

                /** Event parent. */
                parent?: (number|Long|null);

                /** Event dependencies. */
                dependencies: (number|Long)[];

                /** Event state. */
                state: doramcap.workflow.v1.Event.State;

                /** Event elapsedNs. */
                elapsedNs: (number|Long);

                /** Event durationNs. */
                durationNs: (number|Long);

                /** Event payloadJson. */
                payloadJson?: (string|null);

                /** Event error. */
                error?: (string|null);

                /** Event displayName. */
                displayName?: (string|null);

                /**
                 * Creates a new Event instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Event instance
                 */
                static create(properties: doramcap.workflow.v1.Event.$Shape): doramcap.workflow.v1.Event & doramcap.workflow.v1.Event.$Shape;
                static create(properties?: doramcap.workflow.v1.Event.$Properties): doramcap.workflow.v1.Event;

                /**
                 * Encodes the specified Event message. Does not implicitly {@link doramcap.workflow.v1.Event.verify|verify} messages.
                 * @param message Event message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.workflow.v1.Event, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Event message, length delimited. Does not implicitly {@link doramcap.workflow.v1.Event.verify|verify} messages.
                 * @param message Event message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.workflow.v1.Event, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Event message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.workflow.v1.Event & doramcap.workflow.v1.Event.$Shape} Event
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.workflow.v1.Event & doramcap.workflow.v1.Event.$Shape;

                /**
                 * Decodes an Event message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.workflow.v1.Event & doramcap.workflow.v1.Event.$Shape} Event
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.workflow.v1.Event & doramcap.workflow.v1.Event.$Shape;

                /**
                 * Creates an Event message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Event
                 */
                static fromObject(object: { [k: string]: any }): doramcap.workflow.v1.Event;

                /**
                 * Creates a plain object from an Event message. Also converts values to other types if specified.
                 * @param message Event
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.workflow.v1.Event, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Event to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Event
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Event {

                /** Properties of an Event. */
                interface $Properties {

                    /** Event version */
                    version?: (number|null);

                    /** Event sequence */
                    sequence?: (number|Long|null);

                    /** Event session */
                    session?: (string|null);

                    /** Event invocation */
                    invocation?: (number|Long|null);

                    /** Event step */
                    step?: (string|null);

                    /** Event stepVersion */
                    stepVersion?: (number|null);

                    /** Event executor */
                    executor?: (string|null);

                    /** Event parent */
                    parent?: (number|Long|null);

                    /** Event dependencies */
                    dependencies?: ((number|Long)[]|null);

                    /** Event state */
                    state?: (doramcap.workflow.v1.Event.State|null);

                    /** Event elapsedNs */
                    elapsedNs?: (number|Long|null);

                    /** Event durationNs */
                    durationNs?: (number|Long|null);

                    /** Event payloadJson */
                    payloadJson?: (string|null);

                    /** Event error */
                    error?: (string|null);

                    /** Event displayName */
                    displayName?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an Event. */
                type $Shape = doramcap.workflow.v1.Event.$Properties;

                /** State enum. */
                enum State {

                    /** UNSPECIFIED value */
                    UNSPECIFIED = 0,

                    /** STARTED value */
                    STARTED = 1,

                    /** SUCCEEDED value */
                    SUCCEEDED = 2,

                    /** FAILED value */
                    FAILED = 3,

                    /** CANCELLED value */
                    CANCELLED = 4,

                    /** PANICKED value */
                    PANICKED = 5
                }
            }
        }
    }

    /** Namespace telemetry. */
    namespace telemetry {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of a Packet.
             * @deprecated Use doramcap.telemetry.v1.Packet.$Properties instead.
             */
            interface IPacket extends doramcap.telemetry.v1.Packet.$Properties {
            }

            /** Represents a Packet. */
            class Packet {

                /**
                 * Constructs a new Packet.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.Packet.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Packet snapshot. */
                snapshot?: (doramcap.telemetry.v1.SnapshotFrame|null);

                /** Packet dictionary. */
                dictionary?: (doramcap.telemetry.v1.DictionaryFrame|null);

                /** Packet body. */
                body?: ("snapshot"|"dictionary");

                /**
                 * Creates a new Packet instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Packet instance
                 */
                static create(properties: doramcap.telemetry.v1.Packet.$Shape): doramcap.telemetry.v1.Packet & doramcap.telemetry.v1.Packet.$Shape;
                static create(properties?: doramcap.telemetry.v1.Packet.$Properties): doramcap.telemetry.v1.Packet;

                /**
                 * Encodes the specified Packet message. Does not implicitly {@link doramcap.telemetry.v1.Packet.verify|verify} messages.
                 * @param message Packet message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.Packet, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Packet message, length delimited. Does not implicitly {@link doramcap.telemetry.v1.Packet.verify|verify} messages.
                 * @param message Packet message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.telemetry.v1.Packet, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Packet message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.Packet & doramcap.telemetry.v1.Packet.$Shape} Packet
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.Packet & doramcap.telemetry.v1.Packet.$Shape;

                /**
                 * Decodes a Packet message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.telemetry.v1.Packet & doramcap.telemetry.v1.Packet.$Shape} Packet
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.telemetry.v1.Packet & doramcap.telemetry.v1.Packet.$Shape;

                /**
                 * Creates a Packet message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Packet
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.Packet;

                /**
                 * Creates a plain object from a Packet message. Also converts values to other types if specified.
                 * @param message Packet
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.Packet, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Packet to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Packet
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Packet {

                /** Properties of a Packet. */
                interface $Properties {

                    /** Packet snapshot */
                    snapshot?: (doramcap.telemetry.v1.SnapshotFrame|null);

                    /** Packet dictionary */
                    dictionary?: (doramcap.telemetry.v1.DictionaryFrame|null);

                    /** Packet body */
                    body?: ("snapshot"|"dictionary");

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Narrowed shape of a Packet. */
                type $Shape = {
                  snapshot?: doramcap.telemetry.v1.SnapshotFrame.$Shape|null;
                  dictionary?: doramcap.telemetry.v1.DictionaryFrame.$Shape|null;
                  $unknowns?: Uint8Array[];
                } & (
                  ({ body?: undefined; snapshot?: null; dictionary?: null }|{ body?: "snapshot"; snapshot: doramcap.telemetry.v1.SnapshotFrame.$Shape; dictionary?: null }|{ body?: "dictionary"; snapshot?: null; dictionary: doramcap.telemetry.v1.DictionaryFrame.$Shape })
                );
            }

            /**
             * Properties of a CpuRecord.
             * @deprecated Use doramcap.telemetry.v1.CpuRecord.$Properties instead.
             */
            interface ICpuRecord extends doramcap.telemetry.v1.CpuRecord.$Properties {
            }

            /** Represents a CpuRecord. */
            class CpuRecord {

                /**
                 * Constructs a new CpuRecord.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.CpuRecord.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** CpuRecord cgroupId. */
                cgroupId: (number|Long);

                /** CpuRecord parentId. */
                parentId: (number|Long);

                /** CpuRecord cpu. */
                cpu: number;

                /** CpuRecord flags. */
                flags: number;

                /** CpuRecord runtimeNs. */
                runtimeNs: (number|Long);

                /** CpuRecord userNs. */
                userNs: (number|Long);

                /** CpuRecord systemNs. */
                systemNs: (number|Long);

                /**
                 * Creates a new CpuRecord instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns CpuRecord instance
                 */
                static create(properties: doramcap.telemetry.v1.CpuRecord.$Shape): doramcap.telemetry.v1.CpuRecord & doramcap.telemetry.v1.CpuRecord.$Shape;
                static create(properties?: doramcap.telemetry.v1.CpuRecord.$Properties): doramcap.telemetry.v1.CpuRecord;

                /**
                 * Encodes the specified CpuRecord message. Does not implicitly {@link doramcap.telemetry.v1.CpuRecord.verify|verify} messages.
                 * @param message CpuRecord message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.CpuRecord, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified CpuRecord message, length delimited. Does not implicitly {@link doramcap.telemetry.v1.CpuRecord.verify|verify} messages.
                 * @param message CpuRecord message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.telemetry.v1.CpuRecord, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a CpuRecord message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.CpuRecord & doramcap.telemetry.v1.CpuRecord.$Shape} CpuRecord
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.CpuRecord & doramcap.telemetry.v1.CpuRecord.$Shape;

                /**
                 * Decodes a CpuRecord message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.telemetry.v1.CpuRecord & doramcap.telemetry.v1.CpuRecord.$Shape} CpuRecord
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.telemetry.v1.CpuRecord & doramcap.telemetry.v1.CpuRecord.$Shape;

                /**
                 * Creates a CpuRecord message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns CpuRecord
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.CpuRecord;

                /**
                 * Creates a plain object from a CpuRecord message. Also converts values to other types if specified.
                 * @param message CpuRecord
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.CpuRecord, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this CpuRecord to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for CpuRecord
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace CpuRecord {

                /** Properties of a CpuRecord. */
                interface $Properties {

                    /** CpuRecord cgroupId */
                    cgroupId?: (number|Long|null);

                    /** CpuRecord parentId */
                    parentId?: (number|Long|null);

                    /** CpuRecord cpu */
                    cpu?: (number|null);

                    /** CpuRecord flags */
                    flags?: (number|null);

                    /** CpuRecord runtimeNs */
                    runtimeNs?: (number|Long|null);

                    /** CpuRecord userNs */
                    userNs?: (number|Long|null);

                    /** CpuRecord systemNs */
                    systemNs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a CpuRecord. */
                type $Shape = doramcap.telemetry.v1.CpuRecord.$Properties;
            }

            /**
             * Properties of a TaskIdentity.
             * @deprecated Use doramcap.telemetry.v1.TaskIdentity.$Properties instead.
             */
            interface ITaskIdentity extends doramcap.telemetry.v1.TaskIdentity.$Properties {
            }

            /** Represents a TaskIdentity. */
            class TaskIdentity {

                /**
                 * Constructs a new TaskIdentity.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.TaskIdentity.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** TaskIdentity startBoottimeNs. */
                startBoottimeNs: (number|Long);

                /** TaskIdentity cgroupId. */
                cgroupId: (number|Long);

                /** TaskIdentity tid. */
                tid: number;

                /** TaskIdentity tgid. */
                tgid: number;

                /** TaskIdentity cpu. */
                cpu: number;

                /** TaskIdentity flags. */
                flags: number;

                /**
                 * Creates a new TaskIdentity instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns TaskIdentity instance
                 */
                static create(properties: doramcap.telemetry.v1.TaskIdentity.$Shape): doramcap.telemetry.v1.TaskIdentity & doramcap.telemetry.v1.TaskIdentity.$Shape;
                static create(properties?: doramcap.telemetry.v1.TaskIdentity.$Properties): doramcap.telemetry.v1.TaskIdentity;

                /**
                 * Encodes the specified TaskIdentity message. Does not implicitly {@link doramcap.telemetry.v1.TaskIdentity.verify|verify} messages.
                 * @param message TaskIdentity message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.TaskIdentity, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified TaskIdentity message, length delimited. Does not implicitly {@link doramcap.telemetry.v1.TaskIdentity.verify|verify} messages.
                 * @param message TaskIdentity message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.telemetry.v1.TaskIdentity, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a TaskIdentity message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.TaskIdentity & doramcap.telemetry.v1.TaskIdentity.$Shape} TaskIdentity
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.TaskIdentity & doramcap.telemetry.v1.TaskIdentity.$Shape;

                /**
                 * Decodes a TaskIdentity message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.telemetry.v1.TaskIdentity & doramcap.telemetry.v1.TaskIdentity.$Shape} TaskIdentity
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.telemetry.v1.TaskIdentity & doramcap.telemetry.v1.TaskIdentity.$Shape;

                /**
                 * Creates a TaskIdentity message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns TaskIdentity
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.TaskIdentity;

                /**
                 * Creates a plain object from a TaskIdentity message. Also converts values to other types if specified.
                 * @param message TaskIdentity
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.TaskIdentity, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this TaskIdentity to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for TaskIdentity
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace TaskIdentity {

                /** Properties of a TaskIdentity. */
                interface $Properties {

                    /** TaskIdentity startBoottimeNs */
                    startBoottimeNs?: (number|Long|null);

                    /** TaskIdentity cgroupId */
                    cgroupId?: (number|Long|null);

                    /** TaskIdentity tid */
                    tid?: (number|null);

                    /** TaskIdentity tgid */
                    tgid?: (number|null);

                    /** TaskIdentity cpu */
                    cpu?: (number|null);

                    /** TaskIdentity flags */
                    flags?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a TaskIdentity. */
                type $Shape = doramcap.telemetry.v1.TaskIdentity.$Properties;
            }

            /**
             * Properties of a TaskRecord.
             * @deprecated Use doramcap.telemetry.v1.TaskRecord.$Properties instead.
             */
            interface ITaskRecord extends doramcap.telemetry.v1.TaskRecord.$Properties {
            }

            /** Represents a TaskRecord. */
            class TaskRecord {

                /**
                 * Constructs a new TaskRecord.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.TaskRecord.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** TaskRecord identity. */
                identity?: (doramcap.telemetry.v1.TaskIdentity|null);

                /** TaskRecord chargedRuntimeNs. */
                chargedRuntimeNs: (number|Long);

                /**
                 * Creates a new TaskRecord instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns TaskRecord instance
                 */
                static create(properties: doramcap.telemetry.v1.TaskRecord.$Shape): doramcap.telemetry.v1.TaskRecord & doramcap.telemetry.v1.TaskRecord.$Shape;
                static create(properties?: doramcap.telemetry.v1.TaskRecord.$Properties): doramcap.telemetry.v1.TaskRecord;

                /**
                 * Encodes the specified TaskRecord message. Does not implicitly {@link doramcap.telemetry.v1.TaskRecord.verify|verify} messages.
                 * @param message TaskRecord message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.TaskRecord, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified TaskRecord message, length delimited. Does not implicitly {@link doramcap.telemetry.v1.TaskRecord.verify|verify} messages.
                 * @param message TaskRecord message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.telemetry.v1.TaskRecord, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a TaskRecord message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.TaskRecord & doramcap.telemetry.v1.TaskRecord.$Shape} TaskRecord
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.TaskRecord & doramcap.telemetry.v1.TaskRecord.$Shape;

                /**
                 * Decodes a TaskRecord message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.telemetry.v1.TaskRecord & doramcap.telemetry.v1.TaskRecord.$Shape} TaskRecord
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.telemetry.v1.TaskRecord & doramcap.telemetry.v1.TaskRecord.$Shape;

                /**
                 * Creates a TaskRecord message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns TaskRecord
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.TaskRecord;

                /**
                 * Creates a plain object from a TaskRecord message. Also converts values to other types if specified.
                 * @param message TaskRecord
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.TaskRecord, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this TaskRecord to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for TaskRecord
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace TaskRecord {

                /** Properties of a TaskRecord. */
                interface $Properties {

                    /** TaskRecord identity */
                    identity?: (doramcap.telemetry.v1.TaskIdentity|null);

                    /** TaskRecord chargedRuntimeNs */
                    chargedRuntimeNs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a TaskRecord. */
                type $Shape = doramcap.telemetry.v1.TaskRecord.$Properties;
            }

            /**
             * Properties of a TaskLosses.
             * @deprecated Use doramcap.telemetry.v1.TaskLosses.$Properties instead.
             */
            interface ITaskLosses extends doramcap.telemetry.v1.TaskLosses.$Properties {
            }

            /** Represents a TaskLosses. */
            class TaskLosses {

                /**
                 * Constructs a new TaskLosses.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.TaskLosses.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** TaskLosses droppedEvents. */
                droppedEvents: (number|Long);

                /** TaskLosses droppedRuntimeNs. */
                droppedRuntimeNs: (number|Long);

                /** TaskLosses identityErrors. */
                identityErrors: (number|Long);

                /**
                 * Creates a new TaskLosses instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns TaskLosses instance
                 */
                static create(properties: doramcap.telemetry.v1.TaskLosses.$Shape): doramcap.telemetry.v1.TaskLosses & doramcap.telemetry.v1.TaskLosses.$Shape;
                static create(properties?: doramcap.telemetry.v1.TaskLosses.$Properties): doramcap.telemetry.v1.TaskLosses;

                /**
                 * Encodes the specified TaskLosses message. Does not implicitly {@link doramcap.telemetry.v1.TaskLosses.verify|verify} messages.
                 * @param message TaskLosses message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.TaskLosses, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified TaskLosses message, length delimited. Does not implicitly {@link doramcap.telemetry.v1.TaskLosses.verify|verify} messages.
                 * @param message TaskLosses message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.telemetry.v1.TaskLosses, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a TaskLosses message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.TaskLosses & doramcap.telemetry.v1.TaskLosses.$Shape} TaskLosses
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.TaskLosses & doramcap.telemetry.v1.TaskLosses.$Shape;

                /**
                 * Decodes a TaskLosses message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.telemetry.v1.TaskLosses & doramcap.telemetry.v1.TaskLosses.$Shape} TaskLosses
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.telemetry.v1.TaskLosses & doramcap.telemetry.v1.TaskLosses.$Shape;

                /**
                 * Creates a TaskLosses message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns TaskLosses
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.TaskLosses;

                /**
                 * Creates a plain object from a TaskLosses message. Also converts values to other types if specified.
                 * @param message TaskLosses
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.TaskLosses, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this TaskLosses to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for TaskLosses
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace TaskLosses {

                /** Properties of a TaskLosses. */
                interface $Properties {

                    /** TaskLosses droppedEvents */
                    droppedEvents?: (number|Long|null);

                    /** TaskLosses droppedRuntimeNs */
                    droppedRuntimeNs?: (number|Long|null);

                    /** TaskLosses identityErrors */
                    identityErrors?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a TaskLosses. */
                type $Shape = doramcap.telemetry.v1.TaskLosses.$Properties;
            }

            /**
             * Properties of a Snapshot.
             * @deprecated Use doramcap.telemetry.v1.Snapshot.$Properties instead.
             */
            interface ISnapshot extends doramcap.telemetry.v1.Snapshot.$Properties {
            }

            /** Represents a Snapshot. */
            class Snapshot {

                /**
                 * Constructs a new Snapshot.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.Snapshot.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Snapshot schemaVersion. */
                schemaVersion: number;

                /** Snapshot sequence. */
                sequence: (number|Long);

                /** Snapshot captureUnixNs. */
                captureUnixNs: (number|Long);

                /** Snapshot monotonicStartNs. */
                monotonicStartNs: (number|Long);

                /** Snapshot monotonicEndNs. */
                monotonicEndNs: (number|Long);

                /** Snapshot droppedBatches. */
                droppedBatches: (number|Long);

                /** Snapshot capacityOverflow. */
                capacityOverflow: (number|Long);

                /** Snapshot records. */
                records: doramcap.telemetry.v1.CpuRecord[];

                /** Snapshot taskRecords. */
                taskRecords: doramcap.telemetry.v1.TaskRecord[];

                /** Snapshot taskLosses. */
                taskLosses?: (doramcap.telemetry.v1.TaskLosses|null);

                /**
                 * Creates a new Snapshot instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Snapshot instance
                 */
                static create(properties: doramcap.telemetry.v1.Snapshot.$Shape): doramcap.telemetry.v1.Snapshot & doramcap.telemetry.v1.Snapshot.$Shape;
                static create(properties?: doramcap.telemetry.v1.Snapshot.$Properties): doramcap.telemetry.v1.Snapshot;

                /**
                 * Encodes the specified Snapshot message. Does not implicitly {@link doramcap.telemetry.v1.Snapshot.verify|verify} messages.
                 * @param message Snapshot message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.Snapshot, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Snapshot message, length delimited. Does not implicitly {@link doramcap.telemetry.v1.Snapshot.verify|verify} messages.
                 * @param message Snapshot message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.telemetry.v1.Snapshot, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Snapshot message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.Snapshot & doramcap.telemetry.v1.Snapshot.$Shape} Snapshot
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.Snapshot & doramcap.telemetry.v1.Snapshot.$Shape;

                /**
                 * Decodes a Snapshot message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.telemetry.v1.Snapshot & doramcap.telemetry.v1.Snapshot.$Shape} Snapshot
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.telemetry.v1.Snapshot & doramcap.telemetry.v1.Snapshot.$Shape;

                /**
                 * Creates a Snapshot message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Snapshot
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.Snapshot;

                /**
                 * Creates a plain object from a Snapshot message. Also converts values to other types if specified.
                 * @param message Snapshot
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.Snapshot, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Snapshot to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Snapshot
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Snapshot {

                /** Properties of a Snapshot. */
                interface $Properties {

                    /** Snapshot schemaVersion */
                    schemaVersion?: (number|null);

                    /** Snapshot sequence */
                    sequence?: (number|Long|null);

                    /** Snapshot captureUnixNs */
                    captureUnixNs?: (number|Long|null);

                    /** Snapshot monotonicStartNs */
                    monotonicStartNs?: (number|Long|null);

                    /** Snapshot monotonicEndNs */
                    monotonicEndNs?: (number|Long|null);

                    /** Snapshot droppedBatches */
                    droppedBatches?: (number|Long|null);

                    /** Snapshot capacityOverflow */
                    capacityOverflow?: (number|Long|null);

                    /** Snapshot records */
                    records?: (doramcap.telemetry.v1.CpuRecord[]|null);

                    /** Snapshot taskRecords */
                    taskRecords?: (doramcap.telemetry.v1.TaskRecord[]|null);

                    /** Snapshot taskLosses */
                    taskLosses?: (doramcap.telemetry.v1.TaskLosses|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Snapshot. */
                type $Shape = doramcap.telemetry.v1.Snapshot.$Properties;
            }

            /**
             * Properties of a DictionaryEntry.
             * @deprecated Use doramcap.telemetry.v1.DictionaryEntry.$Properties instead.
             */
            interface IDictionaryEntry extends doramcap.telemetry.v1.DictionaryEntry.$Properties {
            }

            /** Represents a DictionaryEntry. */
            class DictionaryEntry {

                /**
                 * Constructs a new DictionaryEntry.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.DictionaryEntry.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** DictionaryEntry id. */
                id: (number|Long);

                /** DictionaryEntry parentId. */
                parentId: (number|Long);

                /** DictionaryEntry path. */
                path?: (string|null);

                /**
                 * Creates a new DictionaryEntry instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns DictionaryEntry instance
                 */
                static create(properties: doramcap.telemetry.v1.DictionaryEntry.$Shape): doramcap.telemetry.v1.DictionaryEntry & doramcap.telemetry.v1.DictionaryEntry.$Shape;
                static create(properties?: doramcap.telemetry.v1.DictionaryEntry.$Properties): doramcap.telemetry.v1.DictionaryEntry;

                /**
                 * Encodes the specified DictionaryEntry message. Does not implicitly {@link doramcap.telemetry.v1.DictionaryEntry.verify|verify} messages.
                 * @param message DictionaryEntry message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.DictionaryEntry, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified DictionaryEntry message, length delimited. Does not implicitly {@link doramcap.telemetry.v1.DictionaryEntry.verify|verify} messages.
                 * @param message DictionaryEntry message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.telemetry.v1.DictionaryEntry, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a DictionaryEntry message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.DictionaryEntry & doramcap.telemetry.v1.DictionaryEntry.$Shape} DictionaryEntry
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.DictionaryEntry & doramcap.telemetry.v1.DictionaryEntry.$Shape;

                /**
                 * Decodes a DictionaryEntry message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.telemetry.v1.DictionaryEntry & doramcap.telemetry.v1.DictionaryEntry.$Shape} DictionaryEntry
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.telemetry.v1.DictionaryEntry & doramcap.telemetry.v1.DictionaryEntry.$Shape;

                /**
                 * Creates a DictionaryEntry message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns DictionaryEntry
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.DictionaryEntry;

                /**
                 * Creates a plain object from a DictionaryEntry message. Also converts values to other types if specified.
                 * @param message DictionaryEntry
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.DictionaryEntry, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this DictionaryEntry to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for DictionaryEntry
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace DictionaryEntry {

                /** Properties of a DictionaryEntry. */
                interface $Properties {

                    /** DictionaryEntry id */
                    id?: (number|Long|null);

                    /** DictionaryEntry parentId */
                    parentId?: (number|Long|null);

                    /** DictionaryEntry path */
                    path?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a DictionaryEntry. */
                type $Shape = doramcap.telemetry.v1.DictionaryEntry.$Properties;
            }

            /**
             * Properties of a SnapshotFrame.
             * @deprecated Use doramcap.telemetry.v1.SnapshotFrame.$Properties instead.
             */
            interface ISnapshotFrame extends doramcap.telemetry.v1.SnapshotFrame.$Properties {
            }

            /** Represents a SnapshotFrame. */
            class SnapshotFrame {

                /**
                 * Constructs a new SnapshotFrame.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.SnapshotFrame.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** SnapshotFrame version. */
                version: number;

                /** SnapshotFrame snapshot. */
                snapshot?: (doramcap.telemetry.v1.Snapshot|null);

                /** SnapshotFrame source. */
                source: string;

                /** SnapshotFrame taskCollectionEnabled. */
                taskCollectionEnabled: boolean;

                /** SnapshotFrame taskCapacity. */
                taskCapacity: (number|Long);

                /** SnapshotFrame sessionId. */
                sessionId: string;

                /** SnapshotFrame dictionaryRevision. */
                dictionaryRevision: (number|Long);

                /** SnapshotFrame dictionaryFull. */
                dictionaryFull: boolean;

                /** SnapshotFrame dictionaryOverflow. */
                dictionaryOverflow: (number|Long);

                /** SnapshotFrame dictionary. */
                dictionary: doramcap.telemetry.v1.DictionaryEntry[];

                /**
                 * Creates a new SnapshotFrame instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns SnapshotFrame instance
                 */
                static create(properties: doramcap.telemetry.v1.SnapshotFrame.$Shape): doramcap.telemetry.v1.SnapshotFrame & doramcap.telemetry.v1.SnapshotFrame.$Shape;
                static create(properties?: doramcap.telemetry.v1.SnapshotFrame.$Properties): doramcap.telemetry.v1.SnapshotFrame;

                /**
                 * Encodes the specified SnapshotFrame message. Does not implicitly {@link doramcap.telemetry.v1.SnapshotFrame.verify|verify} messages.
                 * @param message SnapshotFrame message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.SnapshotFrame, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified SnapshotFrame message, length delimited. Does not implicitly {@link doramcap.telemetry.v1.SnapshotFrame.verify|verify} messages.
                 * @param message SnapshotFrame message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.telemetry.v1.SnapshotFrame, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a SnapshotFrame message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.SnapshotFrame & doramcap.telemetry.v1.SnapshotFrame.$Shape} SnapshotFrame
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.SnapshotFrame & doramcap.telemetry.v1.SnapshotFrame.$Shape;

                /**
                 * Decodes a SnapshotFrame message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.telemetry.v1.SnapshotFrame & doramcap.telemetry.v1.SnapshotFrame.$Shape} SnapshotFrame
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.telemetry.v1.SnapshotFrame & doramcap.telemetry.v1.SnapshotFrame.$Shape;

                /**
                 * Creates a SnapshotFrame message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns SnapshotFrame
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.SnapshotFrame;

                /**
                 * Creates a plain object from a SnapshotFrame message. Also converts values to other types if specified.
                 * @param message SnapshotFrame
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.SnapshotFrame, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this SnapshotFrame to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for SnapshotFrame
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace SnapshotFrame {

                /** Properties of a SnapshotFrame. */
                interface $Properties {

                    /** SnapshotFrame version */
                    version?: (number|null);

                    /** SnapshotFrame snapshot */
                    snapshot?: (doramcap.telemetry.v1.Snapshot|null);

                    /** SnapshotFrame source */
                    source?: (string|null);

                    /** SnapshotFrame taskCollectionEnabled */
                    taskCollectionEnabled?: (boolean|null);

                    /** SnapshotFrame taskCapacity */
                    taskCapacity?: (number|Long|null);

                    /** SnapshotFrame sessionId */
                    sessionId?: (string|null);

                    /** SnapshotFrame dictionaryRevision */
                    dictionaryRevision?: (number|Long|null);

                    /** SnapshotFrame dictionaryFull */
                    dictionaryFull?: (boolean|null);

                    /** SnapshotFrame dictionaryOverflow */
                    dictionaryOverflow?: (number|Long|null);

                    /** SnapshotFrame dictionary */
                    dictionary?: (doramcap.telemetry.v1.DictionaryEntry[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a SnapshotFrame. */
                type $Shape = doramcap.telemetry.v1.SnapshotFrame.$Properties;
            }

            /**
             * Properties of a DictionaryFrame.
             * @deprecated Use doramcap.telemetry.v1.DictionaryFrame.$Properties instead.
             */
            interface IDictionaryFrame extends doramcap.telemetry.v1.DictionaryFrame.$Properties {
            }

            /** Represents a DictionaryFrame. */
            class DictionaryFrame {

                /**
                 * Constructs a new DictionaryFrame.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.DictionaryFrame.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** DictionaryFrame version. */
                version: number;

                /** DictionaryFrame sessionId. */
                sessionId: string;

                /** DictionaryFrame revision. */
                revision: (number|Long);

                /** DictionaryFrame overflow. */
                overflow: (number|Long);

                /** DictionaryFrame entries. */
                entries: doramcap.telemetry.v1.DictionaryEntry[];

                /** DictionaryFrame captureUnixNs. */
                captureUnixNs: (number|Long);

                /**
                 * Creates a new DictionaryFrame instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns DictionaryFrame instance
                 */
                static create(properties: doramcap.telemetry.v1.DictionaryFrame.$Shape): doramcap.telemetry.v1.DictionaryFrame & doramcap.telemetry.v1.DictionaryFrame.$Shape;
                static create(properties?: doramcap.telemetry.v1.DictionaryFrame.$Properties): doramcap.telemetry.v1.DictionaryFrame;

                /**
                 * Encodes the specified DictionaryFrame message. Does not implicitly {@link doramcap.telemetry.v1.DictionaryFrame.verify|verify} messages.
                 * @param message DictionaryFrame message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.DictionaryFrame, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified DictionaryFrame message, length delimited. Does not implicitly {@link doramcap.telemetry.v1.DictionaryFrame.verify|verify} messages.
                 * @param message DictionaryFrame message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.telemetry.v1.DictionaryFrame, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a DictionaryFrame message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.DictionaryFrame & doramcap.telemetry.v1.DictionaryFrame.$Shape} DictionaryFrame
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.DictionaryFrame & doramcap.telemetry.v1.DictionaryFrame.$Shape;

                /**
                 * Decodes a DictionaryFrame message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.telemetry.v1.DictionaryFrame & doramcap.telemetry.v1.DictionaryFrame.$Shape} DictionaryFrame
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.telemetry.v1.DictionaryFrame & doramcap.telemetry.v1.DictionaryFrame.$Shape;

                /**
                 * Creates a DictionaryFrame message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns DictionaryFrame
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.DictionaryFrame;

                /**
                 * Creates a plain object from a DictionaryFrame message. Also converts values to other types if specified.
                 * @param message DictionaryFrame
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.DictionaryFrame, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this DictionaryFrame to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for DictionaryFrame
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace DictionaryFrame {

                /** Properties of a DictionaryFrame. */
                interface $Properties {

                    /** DictionaryFrame version */
                    version?: (number|null);

                    /** DictionaryFrame sessionId */
                    sessionId?: (string|null);

                    /** DictionaryFrame revision */
                    revision?: (number|Long|null);

                    /** DictionaryFrame overflow */
                    overflow?: (number|Long|null);

                    /** DictionaryFrame entries */
                    entries?: (doramcap.telemetry.v1.DictionaryEntry[]|null);

                    /** DictionaryFrame captureUnixNs */
                    captureUnixNs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a DictionaryFrame. */
                type $Shape = doramcap.telemetry.v1.DictionaryFrame.$Properties;
            }
        }
    }

    /** Namespace fault. */
    namespace fault {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of an AggregationPublication.
             * @deprecated Use doramcap.fault.v1.AggregationPublication.$Properties instead.
             */
            interface IAggregationPublication extends doramcap.fault.v1.AggregationPublication.$Properties {
            }

            /** Represents an AggregationPublication. */
            class AggregationPublication {

                /**
                 * Constructs a new AggregationPublication.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.AggregationPublication.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** AggregationPublication version. */
                version: number;

                /** AggregationPublication publishedUnixNs. */
                publishedUnixNs?: (number|Long|null);

                /** AggregationPublication emittedAtNs. */
                emittedAtNs: (number|Long);

                /** AggregationPublication updates. */
                updates?: (doramcap.fault.v1.UpdateBatch|null);

                /** AggregationPublication snapshot. */
                snapshot?: (doramcap.fault.v1.AggregationSnapshot|null);

                /** AggregationPublication payload. */
                payload?: ("updates"|"snapshot");

                /**
                 * Creates a new AggregationPublication instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns AggregationPublication instance
                 */
                static create(properties: doramcap.fault.v1.AggregationPublication.$Shape): doramcap.fault.v1.AggregationPublication & doramcap.fault.v1.AggregationPublication.$Shape;
                static create(properties?: doramcap.fault.v1.AggregationPublication.$Properties): doramcap.fault.v1.AggregationPublication;

                /**
                 * Encodes the specified AggregationPublication message. Does not implicitly {@link doramcap.fault.v1.AggregationPublication.verify|verify} messages.
                 * @param message AggregationPublication message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.AggregationPublication, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified AggregationPublication message, length delimited. Does not implicitly {@link doramcap.fault.v1.AggregationPublication.verify|verify} messages.
                 * @param message AggregationPublication message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.AggregationPublication, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an AggregationPublication message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.AggregationPublication & doramcap.fault.v1.AggregationPublication.$Shape} AggregationPublication
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.AggregationPublication & doramcap.fault.v1.AggregationPublication.$Shape;

                /**
                 * Decodes an AggregationPublication message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.AggregationPublication & doramcap.fault.v1.AggregationPublication.$Shape} AggregationPublication
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.AggregationPublication & doramcap.fault.v1.AggregationPublication.$Shape;

                /**
                 * Creates an AggregationPublication message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns AggregationPublication
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.AggregationPublication;

                /**
                 * Creates a plain object from an AggregationPublication message. Also converts values to other types if specified.
                 * @param message AggregationPublication
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.AggregationPublication, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this AggregationPublication to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for AggregationPublication
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace AggregationPublication {

                /** Properties of an AggregationPublication. */
                interface $Properties {

                    /** AggregationPublication version */
                    version?: (number|null);

                    /** AggregationPublication publishedUnixNs */
                    publishedUnixNs?: (number|Long|null);

                    /** AggregationPublication emittedAtNs */
                    emittedAtNs?: (number|Long|null);

                    /** AggregationPublication updates */
                    updates?: (doramcap.fault.v1.UpdateBatch|null);

                    /** AggregationPublication snapshot */
                    snapshot?: (doramcap.fault.v1.AggregationSnapshot|null);

                    /** AggregationPublication payload */
                    payload?: ("updates"|"snapshot");

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Narrowed shape of an AggregationPublication. */
                type $Shape = {
                  version?: number|null;
                  publishedUnixNs?: number|Long|null;
                  emittedAtNs?: number|Long|null;
                  updates?: doramcap.fault.v1.UpdateBatch.$Shape|null;
                  snapshot?: doramcap.fault.v1.AggregationSnapshot.$Shape|null;
                  $unknowns?: Uint8Array[];
                } & (
                  ({ payload?: undefined; updates?: null; snapshot?: null }|{ payload?: "updates"; updates: doramcap.fault.v1.UpdateBatch.$Shape; snapshot?: null }|{ payload?: "snapshot"; updates?: null; snapshot: doramcap.fault.v1.AggregationSnapshot.$Shape })
                );
            }

            /** Health enum. */
            enum Health {

                /** HEALTH_UNSPECIFIED value */
                HEALTH_UNSPECIFIED = 0,

                /** HEALTH_UNKNOWN value */
                HEALTH_UNKNOWN = 1,

                /** HEALTH_HEALTHY value */
                HEALTH_HEALTHY = 2,

                /** HEALTH_ACTIVE value */
                HEALTH_ACTIVE = 3,

                /** HEALTH_RECOVERING value */
                HEALTH_RECOVERING = 4
            }

            /** Freshness enum. */
            enum Freshness {

                /** FRESHNESS_UNSPECIFIED value */
                FRESHNESS_UNSPECIFIED = 0,

                /** FRESHNESS_UNKNOWN value */
                FRESHNESS_UNKNOWN = 1,

                /** FRESHNESS_FRESH value */
                FRESHNESS_FRESH = 2,

                /** FRESHNESS_STALE value */
                FRESHNESS_STALE = 3
            }

            /** UpdateKind enum. */
            enum UpdateKind {

                /** UPDATE_KIND_UNSPECIFIED value */
                UPDATE_KIND_UNSPECIFIED = 0,

                /** UPDATE_KIND_REGISTERED value */
                UPDATE_KIND_REGISTERED = 1,

                /** UPDATE_KIND_OBSERVATION value */
                UPDATE_KIND_OBSERVATION = 2,

                /** UPDATE_KIND_TRANSITION value */
                UPDATE_KIND_TRANSITION = 3,

                /** UPDATE_KIND_RECONCILED value */
                UPDATE_KIND_RECONCILED = 4,

                /** UPDATE_KIND_LATE value */
                UPDATE_KIND_LATE = 5,

                /** UPDATE_KIND_EVENT value */
                UPDATE_KIND_EVENT = 6,

                /** UPDATE_KIND_FRESHNESS value */
                UPDATE_KIND_FRESHNESS = 7,

                /** UPDATE_KIND_SOURCE value */
                UPDATE_KIND_SOURCE = 8,

                /** UPDATE_KIND_RETIRED value */
                UPDATE_KIND_RETIRED = 9
            }

            /**
             * Properties of a State.
             * @deprecated Use doramcap.fault.v1.State.$Properties instead.
             */
            interface IState extends doramcap.fault.v1.State.$Properties {
            }

            /** Represents a State. */
            class State {

                /**
                 * Constructs a new State.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.State.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** State key. */
                key?: (doramcap.fault.v1.Key|null);

                /** State definition. */
                definition?: (doramcap.fault.v1.Definition|null);

                /** State health. */
                health: doramcap.fault.v1.Health;

                /** State freshness. */
                freshness: doramcap.fault.v1.Freshness;

                /** State latest. */
                latest?: (doramcap.fault.v1.Observation|null);

                /** State revision. */
                revision: (number|Long);

                /**
                 * Creates a new State instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns State instance
                 */
                static create(properties: doramcap.fault.v1.State.$Shape): doramcap.fault.v1.State & doramcap.fault.v1.State.$Shape;
                static create(properties?: doramcap.fault.v1.State.$Properties): doramcap.fault.v1.State;

                /**
                 * Encodes the specified State message. Does not implicitly {@link doramcap.fault.v1.State.verify|verify} messages.
                 * @param message State message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.State, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified State message, length delimited. Does not implicitly {@link doramcap.fault.v1.State.verify|verify} messages.
                 * @param message State message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.State, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a State message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.State & doramcap.fault.v1.State.$Shape} State
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.State & doramcap.fault.v1.State.$Shape;

                /**
                 * Decodes a State message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.State & doramcap.fault.v1.State.$Shape} State
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.State & doramcap.fault.v1.State.$Shape;

                /**
                 * Creates a State message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns State
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.State;

                /**
                 * Creates a plain object from a State message. Also converts values to other types if specified.
                 * @param message State
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.State, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this State to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for State
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace State {

                /** Properties of a State. */
                interface $Properties {

                    /** State key */
                    key?: (doramcap.fault.v1.Key|null);

                    /** State definition */
                    definition?: (doramcap.fault.v1.Definition|null);

                    /** State health */
                    health?: (doramcap.fault.v1.Health|null);

                    /** State freshness */
                    freshness?: (doramcap.fault.v1.Freshness|null);

                    /** State latest */
                    latest?: (doramcap.fault.v1.Observation|null);

                    /** State revision */
                    revision?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a State. */
                type $Shape = doramcap.fault.v1.State.$Properties;
            }

            /**
             * Properties of an Update.
             * @deprecated Use doramcap.fault.v1.Update.$Properties instead.
             */
            interface IUpdate extends doramcap.fault.v1.Update.$Properties {
            }

            /** Represents an Update. */
            class Update {

                /**
                 * Constructs a new Update.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.Update.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Update epoch. */
                epoch: Uint8Array;

                /** Update revision. */
                revision: (number|Long);

                /** Update kind. */
                kind: doramcap.fault.v1.UpdateKind;

                /** Update key. */
                key?: (doramcap.fault.v1.Key|null);

                /** Update state. */
                state?: (doramcap.fault.v1.State|null);

                /** Update observation. */
                observation?: (doramcap.fault.v1.Observation|null);

                /** Update source. */
                source: Uint8Array;

                /** Update connected. */
                connected: boolean;

                /** Update losses. */
                losses?: (doramcap.fault.v1.Losses|null);

                /**
                 * Creates a new Update instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Update instance
                 */
                static create(properties: doramcap.fault.v1.Update.$Shape): doramcap.fault.v1.Update & doramcap.fault.v1.Update.$Shape;
                static create(properties?: doramcap.fault.v1.Update.$Properties): doramcap.fault.v1.Update;

                /**
                 * Encodes the specified Update message. Does not implicitly {@link doramcap.fault.v1.Update.verify|verify} messages.
                 * @param message Update message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.Update, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Update message, length delimited. Does not implicitly {@link doramcap.fault.v1.Update.verify|verify} messages.
                 * @param message Update message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.Update, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Update message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.Update & doramcap.fault.v1.Update.$Shape} Update
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.Update & doramcap.fault.v1.Update.$Shape;

                /**
                 * Decodes an Update message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.Update & doramcap.fault.v1.Update.$Shape} Update
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.Update & doramcap.fault.v1.Update.$Shape;

                /**
                 * Creates an Update message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Update
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.Update;

                /**
                 * Creates a plain object from an Update message. Also converts values to other types if specified.
                 * @param message Update
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.Update, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Update to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Update
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Update {

                /** Properties of an Update. */
                interface $Properties {

                    /** Update epoch */
                    epoch?: (Uint8Array|null);

                    /** Update revision */
                    revision?: (number|Long|null);

                    /** Update kind */
                    kind?: (doramcap.fault.v1.UpdateKind|null);

                    /** Update key */
                    key?: (doramcap.fault.v1.Key|null);

                    /** Update state */
                    state?: (doramcap.fault.v1.State|null);

                    /** Update observation */
                    observation?: (doramcap.fault.v1.Observation|null);

                    /** Update source */
                    source?: (Uint8Array|null);

                    /** Update connected */
                    connected?: (boolean|null);

                    /** Update losses */
                    losses?: (doramcap.fault.v1.Losses|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an Update. */
                type $Shape = doramcap.fault.v1.Update.$Properties;
            }

            /**
             * Properties of a Stats.
             * @deprecated Use doramcap.fault.v1.Stats.$Properties instead.
             */
            interface IStats extends doramcap.fault.v1.Stats.$Properties {
            }

            /** Represents a Stats. */
            class Stats {

                /**
                 * Constructs a new Stats.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.Stats.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Stats droppedUpdates. */
                droppedUpdates: (number|Long);

                /** Stats rejectedInputs. */
                rejectedInputs: (number|Long);

                /** Stats tooOld. */
                tooOld: (number|Long);

                /**
                 * Creates a new Stats instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Stats instance
                 */
                static create(properties: doramcap.fault.v1.Stats.$Shape): doramcap.fault.v1.Stats & doramcap.fault.v1.Stats.$Shape;
                static create(properties?: doramcap.fault.v1.Stats.$Properties): doramcap.fault.v1.Stats;

                /**
                 * Encodes the specified Stats message. Does not implicitly {@link doramcap.fault.v1.Stats.verify|verify} messages.
                 * @param message Stats message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.Stats, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Stats message, length delimited. Does not implicitly {@link doramcap.fault.v1.Stats.verify|verify} messages.
                 * @param message Stats message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.Stats, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Stats message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.Stats & doramcap.fault.v1.Stats.$Shape} Stats
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.Stats & doramcap.fault.v1.Stats.$Shape;

                /**
                 * Decodes a Stats message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.Stats & doramcap.fault.v1.Stats.$Shape} Stats
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.Stats & doramcap.fault.v1.Stats.$Shape;

                /**
                 * Creates a Stats message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Stats
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.Stats;

                /**
                 * Creates a plain object from a Stats message. Also converts values to other types if specified.
                 * @param message Stats
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.Stats, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Stats to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Stats
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Stats {

                /** Properties of a Stats. */
                interface $Properties {

                    /** Stats droppedUpdates */
                    droppedUpdates?: (number|Long|null);

                    /** Stats rejectedInputs */
                    rejectedInputs?: (number|Long|null);

                    /** Stats tooOld */
                    tooOld?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Stats. */
                type $Shape = doramcap.fault.v1.Stats.$Properties;
            }

            /**
             * Properties of an UpdateBatch.
             * @deprecated Use doramcap.fault.v1.UpdateBatch.$Properties instead.
             */
            interface IUpdateBatch extends doramcap.fault.v1.UpdateBatch.$Properties {
            }

            /** Represents an UpdateBatch. */
            class UpdateBatch {

                /**
                 * Constructs a new UpdateBatch.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.UpdateBatch.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** UpdateBatch epoch. */
                epoch: Uint8Array;

                /** UpdateBatch updates. */
                updates: doramcap.fault.v1.Update[];

                /** UpdateBatch stats. */
                stats?: (doramcap.fault.v1.Stats|null);

                /**
                 * Creates a new UpdateBatch instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns UpdateBatch instance
                 */
                static create(properties: doramcap.fault.v1.UpdateBatch.$Shape): doramcap.fault.v1.UpdateBatch & doramcap.fault.v1.UpdateBatch.$Shape;
                static create(properties?: doramcap.fault.v1.UpdateBatch.$Properties): doramcap.fault.v1.UpdateBatch;

                /**
                 * Encodes the specified UpdateBatch message. Does not implicitly {@link doramcap.fault.v1.UpdateBatch.verify|verify} messages.
                 * @param message UpdateBatch message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.UpdateBatch, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified UpdateBatch message, length delimited. Does not implicitly {@link doramcap.fault.v1.UpdateBatch.verify|verify} messages.
                 * @param message UpdateBatch message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.UpdateBatch, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an UpdateBatch message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.UpdateBatch & doramcap.fault.v1.UpdateBatch.$Shape} UpdateBatch
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.UpdateBatch & doramcap.fault.v1.UpdateBatch.$Shape;

                /**
                 * Decodes an UpdateBatch message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.UpdateBatch & doramcap.fault.v1.UpdateBatch.$Shape} UpdateBatch
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.UpdateBatch & doramcap.fault.v1.UpdateBatch.$Shape;

                /**
                 * Creates an UpdateBatch message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns UpdateBatch
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.UpdateBatch;

                /**
                 * Creates a plain object from an UpdateBatch message. Also converts values to other types if specified.
                 * @param message UpdateBatch
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.UpdateBatch, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this UpdateBatch to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for UpdateBatch
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace UpdateBatch {

                /** Properties of an UpdateBatch. */
                interface $Properties {

                    /** UpdateBatch epoch */
                    epoch?: (Uint8Array|null);

                    /** UpdateBatch updates */
                    updates?: (doramcap.fault.v1.Update[]|null);

                    /** UpdateBatch stats */
                    stats?: (doramcap.fault.v1.Stats|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an UpdateBatch. */
                type $Shape = doramcap.fault.v1.UpdateBatch.$Properties;
            }

            /**
             * Properties of a SourceView.
             * @deprecated Use doramcap.fault.v1.SourceView.$Properties instead.
             */
            interface ISourceView extends doramcap.fault.v1.SourceView.$Properties {
            }

            /** Represents a SourceView. */
            class SourceView {

                /**
                 * Constructs a new SourceView.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.SourceView.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** SourceView source. */
                source: Uint8Array;

                /** SourceView connected. */
                connected: boolean;

                /** SourceView losses. */
                losses?: (doramcap.fault.v1.Losses|null);

                /** SourceView conditions. */
                conditions: doramcap.fault.v1.State[];

                /**
                 * Creates a new SourceView instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns SourceView instance
                 */
                static create(properties: doramcap.fault.v1.SourceView.$Shape): doramcap.fault.v1.SourceView & doramcap.fault.v1.SourceView.$Shape;
                static create(properties?: doramcap.fault.v1.SourceView.$Properties): doramcap.fault.v1.SourceView;

                /**
                 * Encodes the specified SourceView message. Does not implicitly {@link doramcap.fault.v1.SourceView.verify|verify} messages.
                 * @param message SourceView message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.SourceView, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified SourceView message, length delimited. Does not implicitly {@link doramcap.fault.v1.SourceView.verify|verify} messages.
                 * @param message SourceView message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.SourceView, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a SourceView message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.SourceView & doramcap.fault.v1.SourceView.$Shape} SourceView
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.SourceView & doramcap.fault.v1.SourceView.$Shape;

                /**
                 * Decodes a SourceView message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.SourceView & doramcap.fault.v1.SourceView.$Shape} SourceView
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.SourceView & doramcap.fault.v1.SourceView.$Shape;

                /**
                 * Creates a SourceView message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns SourceView
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.SourceView;

                /**
                 * Creates a plain object from a SourceView message. Also converts values to other types if specified.
                 * @param message SourceView
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.SourceView, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this SourceView to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for SourceView
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace SourceView {

                /** Properties of a SourceView. */
                interface $Properties {

                    /** SourceView source */
                    source?: (Uint8Array|null);

                    /** SourceView connected */
                    connected?: (boolean|null);

                    /** SourceView losses */
                    losses?: (doramcap.fault.v1.Losses|null);

                    /** SourceView conditions */
                    conditions?: (doramcap.fault.v1.State[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a SourceView. */
                type $Shape = doramcap.fault.v1.SourceView.$Properties;
            }

            /**
             * Properties of an AggregationSnapshot.
             * @deprecated Use doramcap.fault.v1.AggregationSnapshot.$Properties instead.
             */
            interface IAggregationSnapshot extends doramcap.fault.v1.AggregationSnapshot.$Properties {
            }

            /** Represents an AggregationSnapshot. */
            class AggregationSnapshot {

                /**
                 * Constructs a new AggregationSnapshot.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.AggregationSnapshot.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** AggregationSnapshot epoch. */
                epoch: Uint8Array;

                /** AggregationSnapshot revision. */
                revision: (number|Long);

                /** AggregationSnapshot stats. */
                stats?: (doramcap.fault.v1.Stats|null);

                /** AggregationSnapshot sources. */
                sources: doramcap.fault.v1.SourceView[];

                /**
                 * Creates a new AggregationSnapshot instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns AggregationSnapshot instance
                 */
                static create(properties: doramcap.fault.v1.AggregationSnapshot.$Shape): doramcap.fault.v1.AggregationSnapshot & doramcap.fault.v1.AggregationSnapshot.$Shape;
                static create(properties?: doramcap.fault.v1.AggregationSnapshot.$Properties): doramcap.fault.v1.AggregationSnapshot;

                /**
                 * Encodes the specified AggregationSnapshot message. Does not implicitly {@link doramcap.fault.v1.AggregationSnapshot.verify|verify} messages.
                 * @param message AggregationSnapshot message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.AggregationSnapshot, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified AggregationSnapshot message, length delimited. Does not implicitly {@link doramcap.fault.v1.AggregationSnapshot.verify|verify} messages.
                 * @param message AggregationSnapshot message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.AggregationSnapshot, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an AggregationSnapshot message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.AggregationSnapshot & doramcap.fault.v1.AggregationSnapshot.$Shape} AggregationSnapshot
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.AggregationSnapshot & doramcap.fault.v1.AggregationSnapshot.$Shape;

                /**
                 * Decodes an AggregationSnapshot message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.AggregationSnapshot & doramcap.fault.v1.AggregationSnapshot.$Shape} AggregationSnapshot
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.AggregationSnapshot & doramcap.fault.v1.AggregationSnapshot.$Shape;

                /**
                 * Creates an AggregationSnapshot message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns AggregationSnapshot
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.AggregationSnapshot;

                /**
                 * Creates a plain object from an AggregationSnapshot message. Also converts values to other types if specified.
                 * @param message AggregationSnapshot
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.AggregationSnapshot, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this AggregationSnapshot to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for AggregationSnapshot
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace AggregationSnapshot {

                /** Properties of an AggregationSnapshot. */
                interface $Properties {

                    /** AggregationSnapshot epoch */
                    epoch?: (Uint8Array|null);

                    /** AggregationSnapshot revision */
                    revision?: (number|Long|null);

                    /** AggregationSnapshot stats */
                    stats?: (doramcap.fault.v1.Stats|null);

                    /** AggregationSnapshot sources */
                    sources?: (doramcap.fault.v1.SourceView[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an AggregationSnapshot. */
                type $Shape = doramcap.fault.v1.AggregationSnapshot.$Properties;
            }

            /**
             * Properties of a Key.
             * @deprecated Use doramcap.fault.v1.Key.$Properties instead.
             */
            interface IKey extends doramcap.fault.v1.Key.$Properties {
            }

            /** Represents a Key. */
            class Key {

                /**
                 * Constructs a new Key.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.Key.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Key source. */
                source: Uint8Array;

                /** Key condition. */
                condition: number;

                /**
                 * Creates a new Key instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Key instance
                 */
                static create(properties: doramcap.fault.v1.Key.$Shape): doramcap.fault.v1.Key & doramcap.fault.v1.Key.$Shape;
                static create(properties?: doramcap.fault.v1.Key.$Properties): doramcap.fault.v1.Key;

                /**
                 * Encodes the specified Key message. Does not implicitly {@link doramcap.fault.v1.Key.verify|verify} messages.
                 * @param message Key message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.Key, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Key message, length delimited. Does not implicitly {@link doramcap.fault.v1.Key.verify|verify} messages.
                 * @param message Key message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.Key, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Key message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.Key & doramcap.fault.v1.Key.$Shape} Key
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.Key & doramcap.fault.v1.Key.$Shape;

                /**
                 * Decodes a Key message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.Key & doramcap.fault.v1.Key.$Shape} Key
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.Key & doramcap.fault.v1.Key.$Shape;

                /**
                 * Creates a Key message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Key
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.Key;

                /**
                 * Creates a plain object from a Key message. Also converts values to other types if specified.
                 * @param message Key
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.Key, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Key to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Key
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Key {

                /** Properties of a Key. */
                interface $Properties {

                    /** Key source */
                    source?: (Uint8Array|null);

                    /** Key condition */
                    condition?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Key. */
                type $Shape = doramcap.fault.v1.Key.$Properties;
            }

            /** Mode enum. */
            enum Mode {

                /** MODE_UNSPECIFIED value */
                MODE_UNSPECIFIED = 0,

                /** MODE_STATEFUL value */
                MODE_STATEFUL = 1,

                /** MODE_EVENT_ONLY value */
                MODE_EVENT_ONLY = 2
            }

            /** Severity enum. */
            enum Severity {

                /** SEVERITY_UNSPECIFIED value */
                SEVERITY_UNSPECIFIED = 0,

                /** SEVERITY_WARNING value */
                SEVERITY_WARNING = 1,

                /** SEVERITY_ERROR value */
                SEVERITY_ERROR = 2,

                /** SEVERITY_CRITICAL value */
                SEVERITY_CRITICAL = 3
            }

            /** Outcome enum. */
            enum Outcome {

                /** OUTCOME_UNSPECIFIED value */
                OUTCOME_UNSPECIFIED = 0,

                /** OUTCOME_SUCCESS value */
                OUTCOME_SUCCESS = 1,

                /** OUTCOME_FAILURE value */
                OUTCOME_FAILURE = 2
            }

            /**
             * Properties of a Policy.
             * @deprecated Use doramcap.fault.v1.Policy.$Properties instead.
             */
            interface IPolicy extends doramcap.fault.v1.Policy.$Properties {
            }

            /** Represents a Policy. */
            class Policy {

                /**
                 * Constructs a new Policy.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.Policy.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Policy mode. */
                mode: doramcap.fault.v1.Mode;

                /** Policy deadlineNs. */
                deadlineNs?: (number|Long|null);

                /** Policy recoveryNs. */
                recoveryNs?: (number|Long|null);

                /** Policy successIntervalNs. */
                successIntervalNs?: (number|Long|null);

                /**
                 * Creates a new Policy instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Policy instance
                 */
                static create(properties: doramcap.fault.v1.Policy.$Shape): doramcap.fault.v1.Policy & doramcap.fault.v1.Policy.$Shape;
                static create(properties?: doramcap.fault.v1.Policy.$Properties): doramcap.fault.v1.Policy;

                /**
                 * Encodes the specified Policy message. Does not implicitly {@link doramcap.fault.v1.Policy.verify|verify} messages.
                 * @param message Policy message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.Policy, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Policy message, length delimited. Does not implicitly {@link doramcap.fault.v1.Policy.verify|verify} messages.
                 * @param message Policy message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.Policy, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Policy message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.Policy & doramcap.fault.v1.Policy.$Shape} Policy
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.Policy & doramcap.fault.v1.Policy.$Shape;

                /**
                 * Decodes a Policy message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.Policy & doramcap.fault.v1.Policy.$Shape} Policy
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.Policy & doramcap.fault.v1.Policy.$Shape;

                /**
                 * Creates a Policy message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Policy
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.Policy;

                /**
                 * Creates a plain object from a Policy message. Also converts values to other types if specified.
                 * @param message Policy
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.Policy, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Policy to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Policy
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Policy {

                /** Properties of a Policy. */
                interface $Properties {

                    /** Policy mode */
                    mode?: (doramcap.fault.v1.Mode|null);

                    /** Policy deadlineNs */
                    deadlineNs?: (number|Long|null);

                    /** Policy recoveryNs */
                    recoveryNs?: (number|Long|null);

                    /** Policy successIntervalNs */
                    successIntervalNs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Policy. */
                type $Shape = doramcap.fault.v1.Policy.$Properties;
            }

            /**
             * Properties of a Definition.
             * @deprecated Use doramcap.fault.v1.Definition.$Properties instead.
             */
            interface IDefinition extends doramcap.fault.v1.Definition.$Properties {
            }

            /** Represents a Definition. */
            class Definition {

                /**
                 * Constructs a new Definition.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.Definition.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Definition component. */
                component: string;

                /** Definition resource. */
                resource: string;

                /** Definition code. */
                code: string;

                /** Definition severity. */
                severity: doramcap.fault.v1.Severity;

                /** Definition policy. */
                policy?: (doramcap.fault.v1.Policy|null);

                /**
                 * Creates a new Definition instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Definition instance
                 */
                static create(properties: doramcap.fault.v1.Definition.$Shape): doramcap.fault.v1.Definition & doramcap.fault.v1.Definition.$Shape;
                static create(properties?: doramcap.fault.v1.Definition.$Properties): doramcap.fault.v1.Definition;

                /**
                 * Encodes the specified Definition message. Does not implicitly {@link doramcap.fault.v1.Definition.verify|verify} messages.
                 * @param message Definition message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.Definition, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Definition message, length delimited. Does not implicitly {@link doramcap.fault.v1.Definition.verify|verify} messages.
                 * @param message Definition message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.Definition, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Definition message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.Definition & doramcap.fault.v1.Definition.$Shape} Definition
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.Definition & doramcap.fault.v1.Definition.$Shape;

                /**
                 * Decodes a Definition message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.Definition & doramcap.fault.v1.Definition.$Shape} Definition
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.Definition & doramcap.fault.v1.Definition.$Shape;

                /**
                 * Creates a Definition message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Definition
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.Definition;

                /**
                 * Creates a plain object from a Definition message. Also converts values to other types if specified.
                 * @param message Definition
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.Definition, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Definition to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Definition
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Definition {

                /** Properties of a Definition. */
                interface $Properties {

                    /** Definition component */
                    component?: (string|null);

                    /** Definition resource */
                    resource?: (string|null);

                    /** Definition code */
                    code?: (string|null);

                    /** Definition severity */
                    severity?: (doramcap.fault.v1.Severity|null);

                    /** Definition policy */
                    policy?: (doramcap.fault.v1.Policy|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Definition. */
                type $Shape = doramcap.fault.v1.Definition.$Properties;
            }

            /**
             * Properties of a TraceContext.
             * @deprecated Use doramcap.fault.v1.TraceContext.$Properties instead.
             */
            interface ITraceContext extends doramcap.fault.v1.TraceContext.$Properties {
            }

            /** Represents a TraceContext. */
            class TraceContext {

                /**
                 * Constructs a new TraceContext.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.TraceContext.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** TraceContext traceId. */
                traceId: Uint8Array;

                /** TraceContext spanId. */
                spanId: Uint8Array;

                /**
                 * Creates a new TraceContext instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns TraceContext instance
                 */
                static create(properties: doramcap.fault.v1.TraceContext.$Shape): doramcap.fault.v1.TraceContext & doramcap.fault.v1.TraceContext.$Shape;
                static create(properties?: doramcap.fault.v1.TraceContext.$Properties): doramcap.fault.v1.TraceContext;

                /**
                 * Encodes the specified TraceContext message. Does not implicitly {@link doramcap.fault.v1.TraceContext.verify|verify} messages.
                 * @param message TraceContext message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.TraceContext, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified TraceContext message, length delimited. Does not implicitly {@link doramcap.fault.v1.TraceContext.verify|verify} messages.
                 * @param message TraceContext message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.TraceContext, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a TraceContext message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.TraceContext & doramcap.fault.v1.TraceContext.$Shape} TraceContext
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.TraceContext & doramcap.fault.v1.TraceContext.$Shape;

                /**
                 * Decodes a TraceContext message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.TraceContext & doramcap.fault.v1.TraceContext.$Shape} TraceContext
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.TraceContext & doramcap.fault.v1.TraceContext.$Shape;

                /**
                 * Creates a TraceContext message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns TraceContext
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.TraceContext;

                /**
                 * Creates a plain object from a TraceContext message. Also converts values to other types if specified.
                 * @param message TraceContext
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.TraceContext, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this TraceContext to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for TraceContext
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace TraceContext {

                /** Properties of a TraceContext. */
                interface $Properties {

                    /** TraceContext traceId */
                    traceId?: (Uint8Array|null);

                    /** TraceContext spanId */
                    spanId?: (Uint8Array|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a TraceContext. */
                type $Shape = doramcap.fault.v1.TraceContext.$Properties;
            }

            /**
             * Properties of an Observation.
             * @deprecated Use doramcap.fault.v1.Observation.$Properties instead.
             */
            interface IObservation extends doramcap.fault.v1.Observation.$Properties {
            }

            /** Represents an Observation. */
            class Observation {

                /**
                 * Constructs a new Observation.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.Observation.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Observation key. */
                key?: (doramcap.fault.v1.Key|null);

                /** Observation sequence. */
                sequence: (number|Long);

                /** Observation observedAtNs. */
                observedAtNs: (number|Long);

                /** Observation outcome. */
                outcome: doramcap.fault.v1.Outcome;

                /** Observation successes. */
                successes: (number|Long);

                /** Observation failures. */
                failures: (number|Long);

                /** Observation firstFailureAtNs. */
                firstFailureAtNs?: (number|Long|null);

                /** Observation lastFailureAtNs. */
                lastFailureAtNs?: (number|Long|null);

                /** Observation successSinceNs. */
                successSinceNs?: (number|Long|null);

                /** Observation consecutiveSuccesses. */
                consecutiveSuccesses: (number|Long);

                /** Observation maxSuccessSpanNs. */
                maxSuccessSpanNs: (number|Long);

                /** Observation detail. */
                detail: string;

                /** Observation trace. */
                trace?: (doramcap.fault.v1.TraceContext|null);

                /**
                 * Creates a new Observation instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Observation instance
                 */
                static create(properties: doramcap.fault.v1.Observation.$Shape): doramcap.fault.v1.Observation & doramcap.fault.v1.Observation.$Shape;
                static create(properties?: doramcap.fault.v1.Observation.$Properties): doramcap.fault.v1.Observation;

                /**
                 * Encodes the specified Observation message. Does not implicitly {@link doramcap.fault.v1.Observation.verify|verify} messages.
                 * @param message Observation message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.Observation, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Observation message, length delimited. Does not implicitly {@link doramcap.fault.v1.Observation.verify|verify} messages.
                 * @param message Observation message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.Observation, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Observation message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.Observation & doramcap.fault.v1.Observation.$Shape} Observation
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.Observation & doramcap.fault.v1.Observation.$Shape;

                /**
                 * Decodes an Observation message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.Observation & doramcap.fault.v1.Observation.$Shape} Observation
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.Observation & doramcap.fault.v1.Observation.$Shape;

                /**
                 * Creates an Observation message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Observation
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.Observation;

                /**
                 * Creates a plain object from an Observation message. Also converts values to other types if specified.
                 * @param message Observation
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.Observation, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Observation to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Observation
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Observation {

                /** Properties of an Observation. */
                interface $Properties {

                    /** Observation key */
                    key?: (doramcap.fault.v1.Key|null);

                    /** Observation sequence */
                    sequence?: (number|Long|null);

                    /** Observation observedAtNs */
                    observedAtNs?: (number|Long|null);

                    /** Observation outcome */
                    outcome?: (doramcap.fault.v1.Outcome|null);

                    /** Observation successes */
                    successes?: (number|Long|null);

                    /** Observation failures */
                    failures?: (number|Long|null);

                    /** Observation firstFailureAtNs */
                    firstFailureAtNs?: (number|Long|null);

                    /** Observation lastFailureAtNs */
                    lastFailureAtNs?: (number|Long|null);

                    /** Observation successSinceNs */
                    successSinceNs?: (number|Long|null);

                    /** Observation consecutiveSuccesses */
                    consecutiveSuccesses?: (number|Long|null);

                    /** Observation maxSuccessSpanNs */
                    maxSuccessSpanNs?: (number|Long|null);

                    /** Observation detail */
                    detail?: (string|null);

                    /** Observation trace */
                    trace?: (doramcap.fault.v1.TraceContext|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an Observation. */
                type $Shape = doramcap.fault.v1.Observation.$Properties;
            }

            /**
             * Properties of a Losses.
             * @deprecated Use doramcap.fault.v1.Losses.$Properties instead.
             */
            interface ILosses extends doramcap.fault.v1.Losses.$Properties {
            }

            /** Represents a Losses. */
            class Losses {

                /**
                 * Constructs a new Losses.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.fault.v1.Losses.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Losses failures. */
                failures: (number|Long);

                /** Losses successes. */
                successes: (number|Long);

                /** Losses coalescedSuccesses. */
                coalescedSuccesses: (number|Long);

                /**
                 * Creates a new Losses instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Losses instance
                 */
                static create(properties: doramcap.fault.v1.Losses.$Shape): doramcap.fault.v1.Losses & doramcap.fault.v1.Losses.$Shape;
                static create(properties?: doramcap.fault.v1.Losses.$Properties): doramcap.fault.v1.Losses;

                /**
                 * Encodes the specified Losses message. Does not implicitly {@link doramcap.fault.v1.Losses.verify|verify} messages.
                 * @param message Losses message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.fault.v1.Losses, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Losses message, length delimited. Does not implicitly {@link doramcap.fault.v1.Losses.verify|verify} messages.
                 * @param message Losses message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encodeDelimited(message: doramcap.fault.v1.Losses, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Losses message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.fault.v1.Losses & doramcap.fault.v1.Losses.$Shape} Losses
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.fault.v1.Losses & doramcap.fault.v1.Losses.$Shape;

                /**
                 * Decodes a Losses message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns {doramcap.fault.v1.Losses & doramcap.fault.v1.Losses.$Shape} Losses
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): doramcap.fault.v1.Losses & doramcap.fault.v1.Losses.$Shape;

                /**
                 * Creates a Losses message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Losses
                 */
                static fromObject(object: { [k: string]: any }): doramcap.fault.v1.Losses;

                /**
                 * Creates a plain object from a Losses message. Also converts values to other types if specified.
                 * @param message Losses
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.fault.v1.Losses, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Losses to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Losses
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Losses {

                /** Properties of a Losses. */
                interface $Properties {

                    /** Losses failures */
                    failures?: (number|Long|null);

                    /** Losses successes */
                    successes?: (number|Long|null);

                    /** Losses coalescedSuccesses */
                    coalescedSuccesses?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Losses. */
                type $Shape = doramcap.fault.v1.Losses.$Properties;
            }
        }
    }
}
