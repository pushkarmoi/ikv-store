# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: streaming.proto
# Protobuf Python Version: 4.25.0
"""Generated protocol buffer code."""
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
from google.protobuf.internal import builder as _builder
# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()


from google.protobuf import timestamp_pb2 as google_dot_protobuf_dot_timestamp__pb2
from . import common_pb2 as common__pb2


DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(b'\n\x0fstreaming.proto\x12\nikvschemas\x1a\x1fgoogle/protobuf/timestamp.proto\x1a\x0c\x63ommon.proto\"[\n\x0b\x45ventHeader\x12\x38\n\x0fsourceTimestamp\x18\x01 \x01(\x0b\x32\x1a.google.protobuf.TimestampH\x00\x88\x01\x01\x42\x12\n\x10_sourceTimestamp\"\xd3\x02\n\x0cIKVDataEvent\x12,\n\x0b\x65ventHeader\x18\x01 \x01(\x0b\x32\x17.ikvschemas.EventHeader\x12J\n\x19upsertDocumentFieldsEvent\x18\x02 \x01(\x0b\x32%.ikvschemas.UpsertDocumentFieldsEventH\x00\x12J\n\x19\x64\x65leteDocumentFieldsEvent\x18\x03 \x01(\x0b\x32%.ikvschemas.DeleteDocumentFieldsEventH\x00\x12>\n\x13\x64\x65leteDocumentEvent\x18\x04 \x01(\x0b\x32\x1f.ikvschemas.DeleteDocumentEventH\x00\x12\x34\n\x0e\x64ropFieldEvent\x18\x05 \x01(\x0b\x32\x1a.ikvschemas.DropFieldEventH\x00\x42\x07\n\x05\x65vent\"L\n\x19UpsertDocumentFieldsEvent\x12/\n\x08\x64ocument\x18\x01 \x01(\x0b\x32\x1d.ikvschemas.IKVDocumentOnWire\"f\n\x19\x44\x65leteDocumentFieldsEvent\x12\x31\n\ndocumentId\x18\x01 \x01(\x0b\x32\x1d.ikvschemas.IKVDocumentOnWire\x12\x16\n\x0e\x66ieldsToDelete\x18\x02 \x03(\t\"H\n\x13\x44\x65leteDocumentEvent\x12\x31\n\ndocumentId\x18\x01 \x01(\x0b\x32\x1d.ikvschemas.IKVDocumentOnWire\"T\n\x0e\x44ropFieldEvent\x12\x13\n\x0b\x66ield_names\x18\x01 \x03(\t\x12\x1b\n\x13\x66ield_name_prefixes\x18\x02 \x03(\t\x12\x10\n\x08\x64rop_all\x18\x03 \x01(\x08\x42 \n\x14\x63om.inlineio.schemasZ\x08schemas/b\x06proto3')

_globals = globals()
_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, _globals)
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, 'streaming_pb2', _globals)
if _descriptor._USE_C_DESCRIPTORS == False:
  _globals['DESCRIPTOR']._options = None
  _globals['DESCRIPTOR']._serialized_options = b'\n\024com.inlineio.schemasZ\010schemas/'
  _globals['_EVENTHEADER']._serialized_start=78
  _globals['_EVENTHEADER']._serialized_end=169
  _globals['_IKVDATAEVENT']._serialized_start=172
  _globals['_IKVDATAEVENT']._serialized_end=511
  _globals['_UPSERTDOCUMENTFIELDSEVENT']._serialized_start=513
  _globals['_UPSERTDOCUMENTFIELDSEVENT']._serialized_end=589
  _globals['_DELETEDOCUMENTFIELDSEVENT']._serialized_start=591
  _globals['_DELETEDOCUMENTFIELDSEVENT']._serialized_end=693
  _globals['_DELETEDOCUMENTEVENT']._serialized_start=695
  _globals['_DELETEDOCUMENTEVENT']._serialized_end=767
  _globals['_DROPFIELDEVENT']._serialized_start=769
  _globals['_DROPFIELDEVENT']._serialized_end=853
# @@protoc_insertion_point(module_scope)
