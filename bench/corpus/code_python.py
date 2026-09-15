#!/usr/bin/env python3
"""Fixture module for struct_extract benches."""
from __future__ import annotations

import os

class Handler0:
    """Bench handler 0."""
    def __init__(self, name: str, offset: int = 0) -> None:
        self.name = name
        self.offset = offset

    def process_0(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 0])

def util_0(a: int, b: int = 0, *args: object, **kw: object) -> int:
    return a + b + 0

class Handler1:
    """Bench handler 1."""
    def __init__(self, name: str, offset: int = 1) -> None:
        self.name = name
        self.offset = offset

    def process_1(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 1])

def util_1(a: int, b: int = 1, *args: object, **kw: object) -> int:
    return a + b + 1

class Handler2:
    """Bench handler 2."""
    def __init__(self, name: str, offset: int = 2) -> None:
        self.name = name
        self.offset = offset

    def process_2(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 2])

def util_2(a: int, b: int = 2, *args: object, **kw: object) -> int:
    return a + b + 2

class Handler3:
    """Bench handler 3."""
    def __init__(self, name: str, offset: int = 3) -> None:
        self.name = name
        self.offset = offset

    def process_3(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 3])

def util_3(a: int, b: int = 3, *args: object, **kw: object) -> int:
    return a + b + 3

class Handler4:
    """Bench handler 4."""
    def __init__(self, name: str, offset: int = 4) -> None:
        self.name = name
        self.offset = offset

    def process_4(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 4])

def util_4(a: int, b: int = 4, *args: object, **kw: object) -> int:
    return a + b + 4

class Handler5:
    """Bench handler 5."""
    def __init__(self, name: str, offset: int = 5) -> None:
        self.name = name
        self.offset = offset

    def process_5(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 5])

def util_5(a: int, b: int = 5, *args: object, **kw: object) -> int:
    return a + b + 5

class Handler6:
    """Bench handler 6."""
    def __init__(self, name: str, offset: int = 6) -> None:
        self.name = name
        self.offset = offset

    def process_6(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 6])

def util_6(a: int, b: int = 6, *args: object, **kw: object) -> int:
    return a + b + 6

class Handler7:
    """Bench handler 7."""
    def __init__(self, name: str, offset: int = 7) -> None:
        self.name = name
        self.offset = offset

    def process_7(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 7])

def util_7(a: int, b: int = 7, *args: object, **kw: object) -> int:
    return a + b + 7

class Handler8:
    """Bench handler 8."""
    def __init__(self, name: str, offset: int = 8) -> None:
        self.name = name
        self.offset = offset

    def process_8(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 8])

def util_8(a: int, b: int = 8, *args: object, **kw: object) -> int:
    return a + b + 8

class Handler9:
    """Bench handler 9."""
    def __init__(self, name: str, offset: int = 9) -> None:
        self.name = name
        self.offset = offset

    def process_9(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 9])

def util_9(a: int, b: int = 9, *args: object, **kw: object) -> int:
    return a + b + 9

class Handler10:
    """Bench handler 10."""
    def __init__(self, name: str, offset: int = 10) -> None:
        self.name = name
        self.offset = offset

    def process_10(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 10])

def util_10(a: int, b: int = 10, *args: object, **kw: object) -> int:
    return a + b + 10

class Handler11:
    """Bench handler 11."""
    def __init__(self, name: str, offset: int = 11) -> None:
        self.name = name
        self.offset = offset

    def process_11(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 11])

def util_11(a: int, b: int = 11, *args: object, **kw: object) -> int:
    return a + b + 11

class Handler12:
    """Bench handler 12."""
    def __init__(self, name: str, offset: int = 12) -> None:
        self.name = name
        self.offset = offset

    def process_12(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 12])

def util_12(a: int, b: int = 12, *args: object, **kw: object) -> int:
    return a + b + 12

class Handler13:
    """Bench handler 13."""
    def __init__(self, name: str, offset: int = 13) -> None:
        self.name = name
        self.offset = offset

    def process_13(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 13])

def util_13(a: int, b: int = 13, *args: object, **kw: object) -> int:
    return a + b + 13

class Handler14:
    """Bench handler 14."""
    def __init__(self, name: str, offset: int = 14) -> None:
        self.name = name
        self.offset = offset

    def process_14(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 14])

def util_14(a: int, b: int = 14, *args: object, **kw: object) -> int:
    return a + b + 14

class Handler15:
    """Bench handler 15."""
    def __init__(self, name: str, offset: int = 15) -> None:
        self.name = name
        self.offset = offset

    def process_15(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 15])

def util_15(a: int, b: int = 15, *args: object, **kw: object) -> int:
    return a + b + 15

class Handler16:
    """Bench handler 16."""
    def __init__(self, name: str, offset: int = 16) -> None:
        self.name = name
        self.offset = offset

    def process_16(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 16])

def util_16(a: int, b: int = 16, *args: object, **kw: object) -> int:
    return a + b + 16

class Handler17:
    """Bench handler 17."""
    def __init__(self, name: str, offset: int = 17) -> None:
        self.name = name
        self.offset = offset

    def process_17(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 17])

def util_17(a: int, b: int = 17, *args: object, **kw: object) -> int:
    return a + b + 17

class Handler18:
    """Bench handler 18."""
    def __init__(self, name: str, offset: int = 18) -> None:
        self.name = name
        self.offset = offset

    def process_18(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 18])

def util_18(a: int, b: int = 18, *args: object, **kw: object) -> int:
    return a + b + 18

class Handler19:
    """Bench handler 19."""
    def __init__(self, name: str, offset: int = 19) -> None:
        self.name = name
        self.offset = offset

    def process_19(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 19])

def util_19(a: int, b: int = 19, *args: object, **kw: object) -> int:
    return a + b + 19

class Handler20:
    """Bench handler 20."""
    def __init__(self, name: str, offset: int = 20) -> None:
        self.name = name
        self.offset = offset

    def process_20(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 20])

def util_20(a: int, b: int = 20, *args: object, **kw: object) -> int:
    return a + b + 20

class Handler21:
    """Bench handler 21."""
    def __init__(self, name: str, offset: int = 21) -> None:
        self.name = name
        self.offset = offset

    def process_21(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 21])

def util_21(a: int, b: int = 21, *args: object, **kw: object) -> int:
    return a + b + 21

class Handler22:
    """Bench handler 22."""
    def __init__(self, name: str, offset: int = 22) -> None:
        self.name = name
        self.offset = offset

    def process_22(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 22])

def util_22(a: int, b: int = 22, *args: object, **kw: object) -> int:
    return a + b + 22

class Handler23:
    """Bench handler 23."""
    def __init__(self, name: str, offset: int = 23) -> None:
        self.name = name
        self.offset = offset

    def process_23(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 23])

def util_23(a: int, b: int = 23, *args: object, **kw: object) -> int:
    return a + b + 23

class Handler24:
    """Bench handler 24."""
    def __init__(self, name: str, offset: int = 24) -> None:
        self.name = name
        self.offset = offset

    def process_24(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 24])

def util_24(a: int, b: int = 24, *args: object, **kw: object) -> int:
    return a + b + 24

class Handler25:
    """Bench handler 25."""
    def __init__(self, name: str, offset: int = 25) -> None:
        self.name = name
        self.offset = offset

    def process_25(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 25])

def util_25(a: int, b: int = 25, *args: object, **kw: object) -> int:
    return a + b + 25

class Handler26:
    """Bench handler 26."""
    def __init__(self, name: str, offset: int = 26) -> None:
        self.name = name
        self.offset = offset

    def process_26(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 26])

def util_26(a: int, b: int = 26, *args: object, **kw: object) -> int:
    return a + b + 26

class Handler27:
    """Bench handler 27."""
    def __init__(self, name: str, offset: int = 27) -> None:
        self.name = name
        self.offset = offset

    def process_27(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 27])

def util_27(a: int, b: int = 27, *args: object, **kw: object) -> int:
    return a + b + 27

class Handler28:
    """Bench handler 28."""
    def __init__(self, name: str, offset: int = 28) -> None:
        self.name = name
        self.offset = offset

    def process_28(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 28])

def util_28(a: int, b: int = 28, *args: object, **kw: object) -> int:
    return a + b + 28

class Handler29:
    """Bench handler 29."""
    def __init__(self, name: str, offset: int = 29) -> None:
        self.name = name
        self.offset = offset

    def process_29(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 29])

def util_29(a: int, b: int = 29, *args: object, **kw: object) -> int:
    return a + b + 29

class Handler30:
    """Bench handler 30."""
    def __init__(self, name: str, offset: int = 30) -> None:
        self.name = name
        self.offset = offset

    def process_30(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 30])

def util_30(a: int, b: int = 30, *args: object, **kw: object) -> int:
    return a + b + 30

class Handler31:
    """Bench handler 31."""
    def __init__(self, name: str, offset: int = 31) -> None:
        self.name = name
        self.offset = offset

    def process_31(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 31])

def util_31(a: int, b: int = 31, *args: object, **kw: object) -> int:
    return a + b + 31

class Handler32:
    """Bench handler 32."""
    def __init__(self, name: str, offset: int = 32) -> None:
        self.name = name
        self.offset = offset

    def process_32(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 32])

def util_32(a: int, b: int = 32, *args: object, **kw: object) -> int:
    return a + b + 32

class Handler33:
    """Bench handler 33."""
    def __init__(self, name: str, offset: int = 33) -> None:
        self.name = name
        self.offset = offset

    def process_33(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 33])

def util_33(a: int, b: int = 33, *args: object, **kw: object) -> int:
    return a + b + 33

class Handler34:
    """Bench handler 34."""
    def __init__(self, name: str, offset: int = 34) -> None:
        self.name = name
        self.offset = offset

    def process_34(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 34])

def util_34(a: int, b: int = 34, *args: object, **kw: object) -> int:
    return a + b + 34

class Handler35:
    """Bench handler 35."""
    def __init__(self, name: str, offset: int = 35) -> None:
        self.name = name
        self.offset = offset

    def process_35(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 35])

def util_35(a: int, b: int = 35, *args: object, **kw: object) -> int:
    return a + b + 35

class Handler36:
    """Bench handler 36."""
    def __init__(self, name: str, offset: int = 36) -> None:
        self.name = name
        self.offset = offset

    def process_36(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 36])

def util_36(a: int, b: int = 36, *args: object, **kw: object) -> int:
    return a + b + 36

class Handler37:
    """Bench handler 37."""
    def __init__(self, name: str, offset: int = 37) -> None:
        self.name = name
        self.offset = offset

    def process_37(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 37])

def util_37(a: int, b: int = 37, *args: object, **kw: object) -> int:
    return a + b + 37

class Handler38:
    """Bench handler 38."""
    def __init__(self, name: str, offset: int = 38) -> None:
        self.name = name
        self.offset = offset

    def process_38(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 38])

def util_38(a: int, b: int = 38, *args: object, **kw: object) -> int:
    return a + b + 38

class Handler39:
    """Bench handler 39."""
    def __init__(self, name: str, offset: int = 39) -> None:
        self.name = name
        self.offset = offset

    def process_39(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 39])

def util_39(a: int, b: int = 39, *args: object, **kw: object) -> int:
    return a + b + 39

class Handler40:
    """Bench handler 40."""
    def __init__(self, name: str, offset: int = 40) -> None:
        self.name = name
        self.offset = offset

    def process_40(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 40])

def util_40(a: int, b: int = 40, *args: object, **kw: object) -> int:
    return a + b + 40

class Handler41:
    """Bench handler 41."""
    def __init__(self, name: str, offset: int = 41) -> None:
        self.name = name
        self.offset = offset

    def process_41(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 41])

def util_41(a: int, b: int = 41, *args: object, **kw: object) -> int:
    return a + b + 41

class Handler42:
    """Bench handler 42."""
    def __init__(self, name: str, offset: int = 42) -> None:
        self.name = name
        self.offset = offset

    def process_42(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 42])

def util_42(a: int, b: int = 42, *args: object, **kw: object) -> int:
    return a + b + 42

class Handler43:
    """Bench handler 43."""
    def __init__(self, name: str, offset: int = 43) -> None:
        self.name = name
        self.offset = offset

    def process_43(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 43])

def util_43(a: int, b: int = 43, *args: object, **kw: object) -> int:
    return a + b + 43

class Handler44:
    """Bench handler 44."""
    def __init__(self, name: str, offset: int = 44) -> None:
        self.name = name
        self.offset = offset

    def process_44(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 44])

def util_44(a: int, b: int = 44, *args: object, **kw: object) -> int:
    return a + b + 44

class Handler45:
    """Bench handler 45."""
    def __init__(self, name: str, offset: int = 45) -> None:
        self.name = name
        self.offset = offset

    def process_45(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 45])

def util_45(a: int, b: int = 45, *args: object, **kw: object) -> int:
    return a + b + 45

class Handler46:
    """Bench handler 46."""
    def __init__(self, name: str, offset: int = 46) -> None:
        self.name = name
        self.offset = offset

    def process_46(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 46])

def util_46(a: int, b: int = 46, *args: object, **kw: object) -> int:
    return a + b + 46

class Handler47:
    """Bench handler 47."""
    def __init__(self, name: str, offset: int = 47) -> None:
        self.name = name
        self.offset = offset

    def process_47(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 47])

def util_47(a: int, b: int = 47, *args: object, **kw: object) -> int:
    return a + b + 47

class Handler48:
    """Bench handler 48."""
    def __init__(self, name: str, offset: int = 48) -> None:
        self.name = name
        self.offset = offset

    def process_48(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 48])

def util_48(a: int, b: int = 48, *args: object, **kw: object) -> int:
    return a + b + 48

class Handler49:
    """Bench handler 49."""
    def __init__(self, name: str, offset: int = 49) -> None:
        self.name = name
        self.offset = offset

    def process_49(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 49])

def util_49(a: int, b: int = 49, *args: object, **kw: object) -> int:
    return a + b + 49

class Handler50:
    """Bench handler 50."""
    def __init__(self, name: str, offset: int = 50) -> None:
        self.name = name
        self.offset = offset

    def process_50(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 50])

def util_50(a: int, b: int = 50, *args: object, **kw: object) -> int:
    return a + b + 50

class Handler51:
    """Bench handler 51."""
    def __init__(self, name: str, offset: int = 51) -> None:
        self.name = name
        self.offset = offset

    def process_51(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 51])

def util_51(a: int, b: int = 51, *args: object, **kw: object) -> int:
    return a + b + 51

class Handler52:
    """Bench handler 52."""
    def __init__(self, name: str, offset: int = 52) -> None:
        self.name = name
        self.offset = offset

    def process_52(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 52])

def util_52(a: int, b: int = 52, *args: object, **kw: object) -> int:
    return a + b + 52

class Handler53:
    """Bench handler 53."""
    def __init__(self, name: str, offset: int = 53) -> None:
        self.name = name
        self.offset = offset

    def process_53(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 53])

def util_53(a: int, b: int = 53, *args: object, **kw: object) -> int:
    return a + b + 53

class Handler54:
    """Bench handler 54."""
    def __init__(self, name: str, offset: int = 54) -> None:
        self.name = name
        self.offset = offset

    def process_54(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 54])

def util_54(a: int, b: int = 54, *args: object, **kw: object) -> int:
    return a + b + 54

class Handler55:
    """Bench handler 55."""
    def __init__(self, name: str, offset: int = 55) -> None:
        self.name = name
        self.offset = offset

    def process_55(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 55])

def util_55(a: int, b: int = 55, *args: object, **kw: object) -> int:
    return a + b + 55

class Handler56:
    """Bench handler 56."""
    def __init__(self, name: str, offset: int = 56) -> None:
        self.name = name
        self.offset = offset

    def process_56(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 56])

def util_56(a: int, b: int = 56, *args: object, **kw: object) -> int:
    return a + b + 56

class Handler57:
    """Bench handler 57."""
    def __init__(self, name: str, offset: int = 57) -> None:
        self.name = name
        self.offset = offset

    def process_57(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 57])

def util_57(a: int, b: int = 57, *args: object, **kw: object) -> int:
    return a + b + 57

class Handler58:
    """Bench handler 58."""
    def __init__(self, name: str, offset: int = 58) -> None:
        self.name = name
        self.offset = offset

    def process_58(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 58])

def util_58(a: int, b: int = 58, *args: object, **kw: object) -> int:
    return a + b + 58

class Handler59:
    """Bench handler 59."""
    def __init__(self, name: str, offset: int = 59) -> None:
        self.name = name
        self.offset = offset

    def process_59(self, data: bytes, depth: int = 0) -> bytes:
        return data[self.offset:] + bytes([depth + 59])

def util_59(a: int, b: int = 59, *args: object, **kw: object) -> int:
    return a + b + 59
