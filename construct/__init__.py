r"""
Construct 2 -- Parsing Made Fun

Homepage:
	https://github.com/construct/construct
    http://construct.readthedocs.org

Hands-on example:
    >>> from construct import *
    >>> s = Struct(
    ...     "a" / Byte,
    ...     "b" / Short,
    ... )
    >>> print s.parse(b"\x01\x02\x03")
    Container:
        a = 1
        b = 515
    >>> s.build(Container(a=1, b=0x0203))
    b"\x01\x02\x03"
"""

import os
from construct.core import *
if os.getenv("CONSTRUCT_USE_RUST"):
    try:
        from construct_rs import Construct as Construct
        from construct_rs import Subconstruct as Subconstruct
        from construct_rs import BitsInteger as BitsInteger
        from construct_rs import BytesInteger as BytesInteger
        from construct_rs import FormatField as FormatField
        from construct_rs import Bit as Bit
        from construct_rs import Nibble as Nibble
        from construct_rs import Octet as Octet
        from construct_rs import Int8ub, Int16ub, Int32ub, Int64ub
        from construct_rs import Int8sb, Int16sb, Int32sb, Int64sb
        from construct_rs import Int8ul, Int16ul, Int32ul, Int64ul
        from construct_rs import Int8sl, Int16sl, Int32sl, Int64sl
        from construct_rs import Int8un, Int16un, Int32un, Int64un
        from construct_rs import Int8sn, Int16sn, Int32sn, Int64sn
        from construct_rs import Byte, Short, Int, Long
        from construct_rs import Float32b, Float32l, Float32n
        from construct_rs import Float64b, Float64l, Float64n
        from construct_rs import Single, Double
        from construct_rs import Int24ub, Int24ul, Int24un
        from construct_rs import Int24sb, Int24sl, Int24sn
    except Exception:
        pass
from construct.expr import *
from construct.debug import *
from construct.version import *
from construct import lib


#===============================================================================
# metadata
#===============================================================================
__author__ = "Arkadiusz Bulski <arek.bulski@gmail.com>, Tomer Filiba <tomerfiliba@gmail.com>, Corbin Simpson <MostAwesomeDude@gmail.com>"
__version__ = version_string

#===============================================================================
# exposed names
#===============================================================================
__all__ = [
    '__author__',
    '__version__',
    'abs_',
    'AdaptationError',
    'Adapter',
    'Aligned',
    'AlignedStruct',
    'Array',
    'Bit',
    'BitsInteger',
    'BitsSwapped',
    'BitStruct',
    'BitwisableString',
    'Bitwise',
    'Byte',
    'Bytes',
    'BytesInteger',
    'ByteSwapped',
    'Bytewise',
    'CancelParsing',
    'Check',
    'CheckError',
    'Checksum',
    'ChecksumError',
    'Compiled',
    'Compressed',
    'Computed',
    'Const',
    'ConstError',
    'Construct',
    'ConstructError',
    'Container',
    'CString',
    'Debugger',
    'Default',
    'Double',
    'Embedded',
    'EmbeddedSwitch',
    'Enum',
    'EnumInteger',
    'EnumIntegerString',
    'Error',
    'ExplicitError',
    'ExprAdapter',
    'ExprSymmetricAdapter',
    'ExprValidator',
    'Filter',
    'FixedSized',
    'Flag',
    'FlagsEnum',
    'FocusedSeq',
    'FormatField',
    'FormatFieldError',
    'FuncPath',
    'globalPrintFalseFlags',
    'globalPrintFullStrings',
    'GreedyBytes',
    'GreedyRange',
    'GreedyString',
    'Hex',
    'HexDump',
    'If',
    'IfThenElse',
    'Index',
    'IndexFieldError',
    'Indexing',
    'Int',
    'IntegerError',
    'Lazy',
    'LazyArray',
    'LazyBound',
    'LazyContainer',
    'LazyListContainer',
    'LazyStruct',
    'len_',
    'lib',
    'list_',
    'ListContainer',
    'Long',
    'Mapping',
    'MappingError',
    'max_',
    'min_',
    'NamedTuple',
    'NamedTupleError',
    'Nibble',
    'NoneOf',
    'NullStripped',
    'NullTerminated',
    'Numpy',
    'obj_',
    'Octet',
    'OneOf',
    'Optional',
    'Padded',
    'PaddedString',
    'Padding',
    'PaddingError',
    'PascalString',
    'Pass',
    'Path',
    'Path2',
    'Peek',
    'Pickled',
    'Pointer',
    'possiblestringencodings',
    'Prefixed',
    'PrefixedArray',
    'Probe',
    'ProcessRotateLeft',
    'ProcessXor',
    'RangeError',
    'RawCopy',
    'Rebuffered',
    'RebufferedBytesIO',
    'Rebuild',
    'release_date',
    'Renamed',
    'RepeatError',
    'RepeatUntil',
    'RestreamData',
    'Restreamed',
    'RestreamedBytesIO',
    'RotationError',
    'Seek',
    'Select',
    'SelectError',
    'Sequence',
    'setGlobalPrintFalseFlags',
    'setGlobalPrintFullStrings',
    'setGlobalPrintPrivateEntries',
    'Short',
    'Single',
    'SizeofError',
    'Slicing',
    'StopFieldError',
    'StopIf',
    'stream_iseof',
    'stream_read',
    'stream_read_entire',
    'stream_seek',
    'stream_size',
    'stream_tell',
    'stream_write',
    'StreamError',
    'StringEncoded',
    'StringError',
    'Struct',
    'Subconstruct',
    'sum_',
    'Switch',
    'SwitchError',
    'SymmetricAdapter',
    'Tell',
    'Terminated',
    'TerminatedError',
    'this',
    'Timestamp',
    'TimestampError',
    'Transformed',
    'Tunnel',
    'Union',
    'UnionError',
    'ValidationError',
    'Validator',
    'VarInt',
    'version',
    'version_string',
]
__all__ += ["Int%s%s%s" % (n,us,bln) for n in (8,16,24,32,64) for us in "us" for bln in "bln"]
__all__ += ["Float%s%s" % (n,bln) for n in (32,64) for bln in "bln"]
