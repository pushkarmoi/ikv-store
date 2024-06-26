package ikvclient

import (
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"

	schemas "github.com/inlinedio/ikv-store/ikv-go-client/schemas"
	"google.golang.org/protobuf/proto"
)

var EMPTY_STRING string = ""

type DefaultIKVReader struct {
	clientoptions *ClientOptions
	native_reader *NativeReaderV2
}

func NewDefaultIKVReader(clientOptions *ClientOptions) (IKVReader, error) {
	if clientOptions == nil {
		return nil, errors.New("clientOptions are required")
	}

	// no assertion on required options
	// will be done by native call
	return &DefaultIKVReader{
		clientoptions: clientOptions,
		native_reader: nil,
	}, nil
}

// Startup. Reader fetches and combines server/client configs
// and opens embedded index via cgo.
func (reader *DefaultIKVReader) Startup() error {
	// dynamic load native IKV binaries
	if reader.native_reader == nil {
		mountdir, exists := reader.clientoptions.config.StringConfigs["mount_directory"]
		if !exists {
			return errors.New("mount_directory is a required client specified option")
		}

		// ensures `mountdir` and all of its parents exists, creates if not.
		bin_manager, err := NewBinaryManager(mountdir)
		if err != nil {
			return err
		}

		dll_path, err := bin_manager.GetPathToNativeBinary()
		if err != nil {
			return err
		}

		reader.native_reader, err = NewNativeReaderV2(dll_path)
		if err != nil {
			return err
		}
	}

	// fetch server supplied options, and override them with client options
	config, err := reader.createIKVConfig()
	if err != nil {
		return err
	}
	config_bytes, err := proto.Marshal(config)
	if err != nil {
		return err
	}

	// open embedded index reader
	err = reader.native_reader.Open(config_bytes)
	if err != nil {
		return fmt.Errorf("cannot initialize reader: %w", err)
	}

	return nil
}

// Shutdown. Reader invokes shutdown sequence on the embedded index
// via cgo.
func (reader *DefaultIKVReader) Shutdown() error {
	if err := reader.native_reader.Close(); err != nil {
		return err
	}

	return nil
}

func (reader *DefaultIKVReader) HealthCheck() (bool, error) {
	return reader.native_reader.HealthCheck("healthcheck")
}

func (reader *DefaultIKVReader) GetBytesValue(primaryKey interface{}, fieldname string) (bool, []byte, error) {
	var nullable_value []byte
	switch typedPrimaryKey := primaryKey.(type) {
	case string:
		nullable_value = reader.native_reader.GetFieldValue(
			[]byte(typedPrimaryKey),
			fieldname)
	case []byte:
		if typedPrimaryKey == nil {
			return false, nil, errors.New("primaryKey can only be a string or []byte")
		}
		nullable_value = reader.native_reader.GetFieldValue(
			typedPrimaryKey,
			fieldname)
	default:
		// also handles the case where primaryKey == nil
		return false, nil, errors.New("primaryKey can only be a string or []byte")
	}

	return nullable_value != nil, nullable_value, nil
}

func (reader *DefaultIKVReader) MultiGetBytesValues(primaryKeys []interface{}, fieldNames []string) ([][]byte, error) {
	if primaryKeys == nil {
		return nil, errors.New("primaryKeys slice cannot be nil")
	}

	if fieldNames == nil {
		return nil, errors.New("fieldNames slice cannot be nil")
	}

	if len(primaryKeys) == 0 || len(fieldNames) == 0 {
		return make([][]byte, 0), nil
	}

	// serialize typed keys
	// calculate capacity
	var capacity = 0
	for _, primaryKey := range primaryKeys {
		switch typedPrimaryKey := primaryKey.(type) {
		case string:
			capacity += 4 + len(typedPrimaryKey)
		case []byte:
			if typedPrimaryKey == nil {
				return nil, errors.New("primaryKey can only be a string or []byte")
			}
			capacity += 4 + len(typedPrimaryKey)
		default:
			// also handles the case where primaryKey == nil
			return nil, errors.New("primaryKey can only be a string or []byte")
		}
	}
	sizePrefixedPrimaryKeys := bytes.NewBuffer(make([]byte, 0, capacity))

	for _, primaryKey := range primaryKeys {
		switch typedPrimaryKey := primaryKey.(type) {
		case string:
			value := []byte(typedPrimaryKey)
			binary.Write(sizePrefixedPrimaryKeys, binary.LittleEndian, int32(len(value)))
			sizePrefixedPrimaryKeys.Write([]byte(typedPrimaryKey))
		case []byte:
			binary.Write(sizePrefixedPrimaryKeys, binary.LittleEndian, int32(len(typedPrimaryKey)))
			sizePrefixedPrimaryKeys.Write(typedPrimaryKey)
		default:
			// also handles the case where primaryKey == nil
			return nil, errors.New("primaryKey can only be a string or []byte")
		}
	}

	return reader.native_reader.MultiGetFieldValues(int32(len(primaryKeys)), sizePrefixedPrimaryKeys.Bytes(), fieldNames)
}

func (reader *DefaultIKVReader) GetStringValue(primaryKey interface{}, fieldname string) (bool, string, error) {
	exists, bytes_value, err := reader.GetBytesValue(primaryKey, fieldname)
	if !exists || err != nil {
		return false, EMPTY_STRING, err
	}

	return true, string(bytes_value), nil
}

func (reader *DefaultIKVReader) createIKVConfig() (*schemas.IKVStoreConfig, error) {
	client, err := NewDefaultIKVWriter(reader.clientoptions)
	if err != nil {
		return nil, fmt.Errorf("cannot fetch server supplied options: %w", err)
	}

	err = client.Startup()
	if err != nil {
		return nil, fmt.Errorf("cannot fetch server supplied options: %w", err)
	}

	config, err := client.serverSuppliedConfig()
	if err != nil {
		return nil, fmt.Errorf("cannot fetch server supplied options: %w", err)
	}

	err = client.Shutdown()
	if err != nil {
		return nil, fmt.Errorf("cannot fetch server supplied options: %w", err)
	}

	if config.StringConfigs == nil {
		config.StringConfigs = make(map[string]string)
	}
	for k, v := range reader.clientoptions.config.StringConfigs {
		config.StringConfigs[k] = v
	}

	if config.IntConfigs == nil {
		config.IntConfigs = make(map[string]int64)
	}
	for k, v := range reader.clientoptions.config.IntConfigs {
		config.IntConfigs[k] = v
	}

	if config.FloatConfigs == nil {
		config.FloatConfigs = make(map[string]float32)
	}
	for k, v := range reader.clientoptions.config.FloatConfigs {
		config.FloatConfigs[k] = v
	}

	if config.BytesConfigs == nil {
		config.BytesConfigs = make(map[string][]byte)
	}
	for k, v := range reader.clientoptions.config.BytesConfigs {
		config.BytesConfigs[k] = v
	}

	if config.BooleanConfigs == nil {
		config.BooleanConfigs = make(map[string]bool)
	}
	for k, v := range reader.clientoptions.config.BooleanConfigs {
		config.BooleanConfigs[k] = v
	}

	return config, nil
}
