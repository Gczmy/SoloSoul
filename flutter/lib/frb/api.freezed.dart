// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'api.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$AccountInfo {

 String get id; String get name; String? get lastAccessed; String? get passwordHint; String? get lastLoginAt; String? get lastOperationAt; String? get lastOperationDesc;
/// Create a copy of AccountInfo
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AccountInfoCopyWith<AccountInfo> get copyWith => _$AccountInfoCopyWithImpl<AccountInfo>(this as AccountInfo, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AccountInfo&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name)&&(identical(other.lastAccessed, lastAccessed) || other.lastAccessed == lastAccessed)&&(identical(other.passwordHint, passwordHint) || other.passwordHint == passwordHint)&&(identical(other.lastLoginAt, lastLoginAt) || other.lastLoginAt == lastLoginAt)&&(identical(other.lastOperationAt, lastOperationAt) || other.lastOperationAt == lastOperationAt)&&(identical(other.lastOperationDesc, lastOperationDesc) || other.lastOperationDesc == lastOperationDesc));
}


@override
int get hashCode => Object.hash(runtimeType,id,name,lastAccessed,passwordHint,lastLoginAt,lastOperationAt,lastOperationDesc);

@override
String toString() {
  return 'AccountInfo(id: $id, name: $name, lastAccessed: $lastAccessed, passwordHint: $passwordHint, lastLoginAt: $lastLoginAt, lastOperationAt: $lastOperationAt, lastOperationDesc: $lastOperationDesc)';
}


}

/// @nodoc
abstract mixin class $AccountInfoCopyWith<$Res>  {
  factory $AccountInfoCopyWith(AccountInfo value, $Res Function(AccountInfo) _then) = _$AccountInfoCopyWithImpl;
@useResult
$Res call({
 String id, String name, String? lastAccessed, String? passwordHint, String? lastLoginAt, String? lastOperationAt, String? lastOperationDesc
});




}
/// @nodoc
class _$AccountInfoCopyWithImpl<$Res>
    implements $AccountInfoCopyWith<$Res> {
  _$AccountInfoCopyWithImpl(this._self, this._then);

  final AccountInfo _self;
  final $Res Function(AccountInfo) _then;

/// Create a copy of AccountInfo
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? id = null,Object? name = null,Object? lastAccessed = freezed,Object? passwordHint = freezed,Object? lastLoginAt = freezed,Object? lastOperationAt = freezed,Object? lastOperationDesc = freezed,}) {
  return _then(_self.copyWith(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,lastAccessed: freezed == lastAccessed ? _self.lastAccessed : lastAccessed // ignore: cast_nullable_to_non_nullable
as String?,passwordHint: freezed == passwordHint ? _self.passwordHint : passwordHint // ignore: cast_nullable_to_non_nullable
as String?,lastLoginAt: freezed == lastLoginAt ? _self.lastLoginAt : lastLoginAt // ignore: cast_nullable_to_non_nullable
as String?,lastOperationAt: freezed == lastOperationAt ? _self.lastOperationAt : lastOperationAt // ignore: cast_nullable_to_non_nullable
as String?,lastOperationDesc: freezed == lastOperationDesc ? _self.lastOperationDesc : lastOperationDesc // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}

}


/// Adds pattern-matching-related methods to [AccountInfo].
extension AccountInfoPatterns on AccountInfo {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _AccountInfo value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _AccountInfo() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _AccountInfo value)  $default,){
final _that = this;
switch (_that) {
case _AccountInfo():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _AccountInfo value)?  $default,){
final _that = this;
switch (_that) {
case _AccountInfo() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String id,  String name,  String? lastAccessed,  String? passwordHint,  String? lastLoginAt,  String? lastOperationAt,  String? lastOperationDesc)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _AccountInfo() when $default != null:
return $default(_that.id,_that.name,_that.lastAccessed,_that.passwordHint,_that.lastLoginAt,_that.lastOperationAt,_that.lastOperationDesc);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String id,  String name,  String? lastAccessed,  String? passwordHint,  String? lastLoginAt,  String? lastOperationAt,  String? lastOperationDesc)  $default,) {final _that = this;
switch (_that) {
case _AccountInfo():
return $default(_that.id,_that.name,_that.lastAccessed,_that.passwordHint,_that.lastLoginAt,_that.lastOperationAt,_that.lastOperationDesc);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String id,  String name,  String? lastAccessed,  String? passwordHint,  String? lastLoginAt,  String? lastOperationAt,  String? lastOperationDesc)?  $default,) {final _that = this;
switch (_that) {
case _AccountInfo() when $default != null:
return $default(_that.id,_that.name,_that.lastAccessed,_that.passwordHint,_that.lastLoginAt,_that.lastOperationAt,_that.lastOperationDesc);case _:
  return null;

}
}

}

/// @nodoc


class _AccountInfo implements AccountInfo {
  const _AccountInfo({required this.id, required this.name, this.lastAccessed, this.passwordHint, this.lastLoginAt, this.lastOperationAt, this.lastOperationDesc});
  

@override final  String id;
@override final  String name;
@override final  String? lastAccessed;
@override final  String? passwordHint;
@override final  String? lastLoginAt;
@override final  String? lastOperationAt;
@override final  String? lastOperationDesc;

/// Create a copy of AccountInfo
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$AccountInfoCopyWith<_AccountInfo> get copyWith => __$AccountInfoCopyWithImpl<_AccountInfo>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _AccountInfo&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name)&&(identical(other.lastAccessed, lastAccessed) || other.lastAccessed == lastAccessed)&&(identical(other.passwordHint, passwordHint) || other.passwordHint == passwordHint)&&(identical(other.lastLoginAt, lastLoginAt) || other.lastLoginAt == lastLoginAt)&&(identical(other.lastOperationAt, lastOperationAt) || other.lastOperationAt == lastOperationAt)&&(identical(other.lastOperationDesc, lastOperationDesc) || other.lastOperationDesc == lastOperationDesc));
}


@override
int get hashCode => Object.hash(runtimeType,id,name,lastAccessed,passwordHint,lastLoginAt,lastOperationAt,lastOperationDesc);

@override
String toString() {
  return 'AccountInfo(id: $id, name: $name, lastAccessed: $lastAccessed, passwordHint: $passwordHint, lastLoginAt: $lastLoginAt, lastOperationAt: $lastOperationAt, lastOperationDesc: $lastOperationDesc)';
}


}

/// @nodoc
abstract mixin class _$AccountInfoCopyWith<$Res> implements $AccountInfoCopyWith<$Res> {
  factory _$AccountInfoCopyWith(_AccountInfo value, $Res Function(_AccountInfo) _then) = __$AccountInfoCopyWithImpl;
@override @useResult
$Res call({
 String id, String name, String? lastAccessed, String? passwordHint, String? lastLoginAt, String? lastOperationAt, String? lastOperationDesc
});




}
/// @nodoc
class __$AccountInfoCopyWithImpl<$Res>
    implements _$AccountInfoCopyWith<$Res> {
  __$AccountInfoCopyWithImpl(this._self, this._then);

  final _AccountInfo _self;
  final $Res Function(_AccountInfo) _then;

/// Create a copy of AccountInfo
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? id = null,Object? name = null,Object? lastAccessed = freezed,Object? passwordHint = freezed,Object? lastLoginAt = freezed,Object? lastOperationAt = freezed,Object? lastOperationDesc = freezed,}) {
  return _then(_AccountInfo(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,lastAccessed: freezed == lastAccessed ? _self.lastAccessed : lastAccessed // ignore: cast_nullable_to_non_nullable
as String?,passwordHint: freezed == passwordHint ? _self.passwordHint : passwordHint // ignore: cast_nullable_to_non_nullable
as String?,lastLoginAt: freezed == lastLoginAt ? _self.lastLoginAt : lastLoginAt // ignore: cast_nullable_to_non_nullable
as String?,lastOperationAt: freezed == lastOperationAt ? _self.lastOperationAt : lastOperationAt // ignore: cast_nullable_to_non_nullable
as String?,lastOperationDesc: freezed == lastOperationDesc ? _self.lastOperationDesc : lastOperationDesc // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc
mixin _$ChangePasswordResult {

 bool get success; String? get error; String? get salt; String? get verifyHash;
/// Create a copy of ChangePasswordResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ChangePasswordResultCopyWith<ChangePasswordResult> get copyWith => _$ChangePasswordResultCopyWithImpl<ChangePasswordResult>(this as ChangePasswordResult, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ChangePasswordResult&&(identical(other.success, success) || other.success == success)&&(identical(other.error, error) || other.error == error)&&(identical(other.salt, salt) || other.salt == salt)&&(identical(other.verifyHash, verifyHash) || other.verifyHash == verifyHash));
}


@override
int get hashCode => Object.hash(runtimeType,success,error,salt,verifyHash);

@override
String toString() {
  return 'ChangePasswordResult(success: $success, error: $error, salt: $salt, verifyHash: $verifyHash)';
}


}

/// @nodoc
abstract mixin class $ChangePasswordResultCopyWith<$Res>  {
  factory $ChangePasswordResultCopyWith(ChangePasswordResult value, $Res Function(ChangePasswordResult) _then) = _$ChangePasswordResultCopyWithImpl;
@useResult
$Res call({
 bool success, String? error, String? salt, String? verifyHash
});




}
/// @nodoc
class _$ChangePasswordResultCopyWithImpl<$Res>
    implements $ChangePasswordResultCopyWith<$Res> {
  _$ChangePasswordResultCopyWithImpl(this._self, this._then);

  final ChangePasswordResult _self;
  final $Res Function(ChangePasswordResult) _then;

/// Create a copy of ChangePasswordResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? success = null,Object? error = freezed,Object? salt = freezed,Object? verifyHash = freezed,}) {
  return _then(_self.copyWith(
success: null == success ? _self.success : success // ignore: cast_nullable_to_non_nullable
as bool,error: freezed == error ? _self.error : error // ignore: cast_nullable_to_non_nullable
as String?,salt: freezed == salt ? _self.salt : salt // ignore: cast_nullable_to_non_nullable
as String?,verifyHash: freezed == verifyHash ? _self.verifyHash : verifyHash // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}

}


/// Adds pattern-matching-related methods to [ChangePasswordResult].
extension ChangePasswordResultPatterns on ChangePasswordResult {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ChangePasswordResult value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ChangePasswordResult() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ChangePasswordResult value)  $default,){
final _that = this;
switch (_that) {
case _ChangePasswordResult():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ChangePasswordResult value)?  $default,){
final _that = this;
switch (_that) {
case _ChangePasswordResult() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( bool success,  String? error,  String? salt,  String? verifyHash)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ChangePasswordResult() when $default != null:
return $default(_that.success,_that.error,_that.salt,_that.verifyHash);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( bool success,  String? error,  String? salt,  String? verifyHash)  $default,) {final _that = this;
switch (_that) {
case _ChangePasswordResult():
return $default(_that.success,_that.error,_that.salt,_that.verifyHash);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( bool success,  String? error,  String? salt,  String? verifyHash)?  $default,) {final _that = this;
switch (_that) {
case _ChangePasswordResult() when $default != null:
return $default(_that.success,_that.error,_that.salt,_that.verifyHash);case _:
  return null;

}
}

}

/// @nodoc


class _ChangePasswordResult implements ChangePasswordResult {
  const _ChangePasswordResult({required this.success, this.error, this.salt, this.verifyHash});
  

@override final  bool success;
@override final  String? error;
@override final  String? salt;
@override final  String? verifyHash;

/// Create a copy of ChangePasswordResult
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ChangePasswordResultCopyWith<_ChangePasswordResult> get copyWith => __$ChangePasswordResultCopyWithImpl<_ChangePasswordResult>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ChangePasswordResult&&(identical(other.success, success) || other.success == success)&&(identical(other.error, error) || other.error == error)&&(identical(other.salt, salt) || other.salt == salt)&&(identical(other.verifyHash, verifyHash) || other.verifyHash == verifyHash));
}


@override
int get hashCode => Object.hash(runtimeType,success,error,salt,verifyHash);

@override
String toString() {
  return 'ChangePasswordResult(success: $success, error: $error, salt: $salt, verifyHash: $verifyHash)';
}


}

/// @nodoc
abstract mixin class _$ChangePasswordResultCopyWith<$Res> implements $ChangePasswordResultCopyWith<$Res> {
  factory _$ChangePasswordResultCopyWith(_ChangePasswordResult value, $Res Function(_ChangePasswordResult) _then) = __$ChangePasswordResultCopyWithImpl;
@override @useResult
$Res call({
 bool success, String? error, String? salt, String? verifyHash
});




}
/// @nodoc
class __$ChangePasswordResultCopyWithImpl<$Res>
    implements _$ChangePasswordResultCopyWith<$Res> {
  __$ChangePasswordResultCopyWithImpl(this._self, this._then);

  final _ChangePasswordResult _self;
  final $Res Function(_ChangePasswordResult) _then;

/// Create a copy of ChangePasswordResult
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? success = null,Object? error = freezed,Object? salt = freezed,Object? verifyHash = freezed,}) {
  return _then(_ChangePasswordResult(
success: null == success ? _self.success : success // ignore: cast_nullable_to_non_nullable
as bool,error: freezed == error ? _self.error : error // ignore: cast_nullable_to_non_nullable
as String?,salt: freezed == salt ? _self.salt : salt // ignore: cast_nullable_to_non_nullable
as String?,verifyHash: freezed == verifyHash ? _self.verifyHash : verifyHash // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc
mixin _$CreateAccountResult {

 bool get success; String? get error; String? get accountId; String? get name; String? get salt; String? get verifyHash;
/// Create a copy of CreateAccountResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CreateAccountResultCopyWith<CreateAccountResult> get copyWith => _$CreateAccountResultCopyWithImpl<CreateAccountResult>(this as CreateAccountResult, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CreateAccountResult&&(identical(other.success, success) || other.success == success)&&(identical(other.error, error) || other.error == error)&&(identical(other.accountId, accountId) || other.accountId == accountId)&&(identical(other.name, name) || other.name == name)&&(identical(other.salt, salt) || other.salt == salt)&&(identical(other.verifyHash, verifyHash) || other.verifyHash == verifyHash));
}


@override
int get hashCode => Object.hash(runtimeType,success,error,accountId,name,salt,verifyHash);

@override
String toString() {
  return 'CreateAccountResult(success: $success, error: $error, accountId: $accountId, name: $name, salt: $salt, verifyHash: $verifyHash)';
}


}

/// @nodoc
abstract mixin class $CreateAccountResultCopyWith<$Res>  {
  factory $CreateAccountResultCopyWith(CreateAccountResult value, $Res Function(CreateAccountResult) _then) = _$CreateAccountResultCopyWithImpl;
@useResult
$Res call({
 bool success, String? error, String? accountId, String? name, String? salt, String? verifyHash
});




}
/// @nodoc
class _$CreateAccountResultCopyWithImpl<$Res>
    implements $CreateAccountResultCopyWith<$Res> {
  _$CreateAccountResultCopyWithImpl(this._self, this._then);

  final CreateAccountResult _self;
  final $Res Function(CreateAccountResult) _then;

/// Create a copy of CreateAccountResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? success = null,Object? error = freezed,Object? accountId = freezed,Object? name = freezed,Object? salt = freezed,Object? verifyHash = freezed,}) {
  return _then(_self.copyWith(
success: null == success ? _self.success : success // ignore: cast_nullable_to_non_nullable
as bool,error: freezed == error ? _self.error : error // ignore: cast_nullable_to_non_nullable
as String?,accountId: freezed == accountId ? _self.accountId : accountId // ignore: cast_nullable_to_non_nullable
as String?,name: freezed == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String?,salt: freezed == salt ? _self.salt : salt // ignore: cast_nullable_to_non_nullable
as String?,verifyHash: freezed == verifyHash ? _self.verifyHash : verifyHash // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}

}


/// Adds pattern-matching-related methods to [CreateAccountResult].
extension CreateAccountResultPatterns on CreateAccountResult {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _CreateAccountResult value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _CreateAccountResult() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _CreateAccountResult value)  $default,){
final _that = this;
switch (_that) {
case _CreateAccountResult():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _CreateAccountResult value)?  $default,){
final _that = this;
switch (_that) {
case _CreateAccountResult() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( bool success,  String? error,  String? accountId,  String? name,  String? salt,  String? verifyHash)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _CreateAccountResult() when $default != null:
return $default(_that.success,_that.error,_that.accountId,_that.name,_that.salt,_that.verifyHash);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( bool success,  String? error,  String? accountId,  String? name,  String? salt,  String? verifyHash)  $default,) {final _that = this;
switch (_that) {
case _CreateAccountResult():
return $default(_that.success,_that.error,_that.accountId,_that.name,_that.salt,_that.verifyHash);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( bool success,  String? error,  String? accountId,  String? name,  String? salt,  String? verifyHash)?  $default,) {final _that = this;
switch (_that) {
case _CreateAccountResult() when $default != null:
return $default(_that.success,_that.error,_that.accountId,_that.name,_that.salt,_that.verifyHash);case _:
  return null;

}
}

}

/// @nodoc


class _CreateAccountResult implements CreateAccountResult {
  const _CreateAccountResult({required this.success, this.error, this.accountId, this.name, this.salt, this.verifyHash});
  

@override final  bool success;
@override final  String? error;
@override final  String? accountId;
@override final  String? name;
@override final  String? salt;
@override final  String? verifyHash;

/// Create a copy of CreateAccountResult
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$CreateAccountResultCopyWith<_CreateAccountResult> get copyWith => __$CreateAccountResultCopyWithImpl<_CreateAccountResult>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _CreateAccountResult&&(identical(other.success, success) || other.success == success)&&(identical(other.error, error) || other.error == error)&&(identical(other.accountId, accountId) || other.accountId == accountId)&&(identical(other.name, name) || other.name == name)&&(identical(other.salt, salt) || other.salt == salt)&&(identical(other.verifyHash, verifyHash) || other.verifyHash == verifyHash));
}


@override
int get hashCode => Object.hash(runtimeType,success,error,accountId,name,salt,verifyHash);

@override
String toString() {
  return 'CreateAccountResult(success: $success, error: $error, accountId: $accountId, name: $name, salt: $salt, verifyHash: $verifyHash)';
}


}

/// @nodoc
abstract mixin class _$CreateAccountResultCopyWith<$Res> implements $CreateAccountResultCopyWith<$Res> {
  factory _$CreateAccountResultCopyWith(_CreateAccountResult value, $Res Function(_CreateAccountResult) _then) = __$CreateAccountResultCopyWithImpl;
@override @useResult
$Res call({
 bool success, String? error, String? accountId, String? name, String? salt, String? verifyHash
});




}
/// @nodoc
class __$CreateAccountResultCopyWithImpl<$Res>
    implements _$CreateAccountResultCopyWith<$Res> {
  __$CreateAccountResultCopyWithImpl(this._self, this._then);

  final _CreateAccountResult _self;
  final $Res Function(_CreateAccountResult) _then;

/// Create a copy of CreateAccountResult
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? success = null,Object? error = freezed,Object? accountId = freezed,Object? name = freezed,Object? salt = freezed,Object? verifyHash = freezed,}) {
  return _then(_CreateAccountResult(
success: null == success ? _self.success : success // ignore: cast_nullable_to_non_nullable
as bool,error: freezed == error ? _self.error : error // ignore: cast_nullable_to_non_nullable
as String?,accountId: freezed == accountId ? _self.accountId : accountId // ignore: cast_nullable_to_non_nullable
as String?,name: freezed == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String?,salt: freezed == salt ? _self.salt : salt // ignore: cast_nullable_to_non_nullable
as String?,verifyHash: freezed == verifyHash ? _self.verifyHash : verifyHash // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc
mixin _$FieldHistoryEntry {

 PropertyValue get value; String get timestamp; String? get source;
/// Create a copy of FieldHistoryEntry
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FieldHistoryEntryCopyWith<FieldHistoryEntry> get copyWith => _$FieldHistoryEntryCopyWithImpl<FieldHistoryEntry>(this as FieldHistoryEntry, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FieldHistoryEntry&&(identical(other.value, value) || other.value == value)&&(identical(other.timestamp, timestamp) || other.timestamp == timestamp)&&(identical(other.source, source) || other.source == source));
}


@override
int get hashCode => Object.hash(runtimeType,value,timestamp,source);

@override
String toString() {
  return 'FieldHistoryEntry(value: $value, timestamp: $timestamp, source: $source)';
}


}

/// @nodoc
abstract mixin class $FieldHistoryEntryCopyWith<$Res>  {
  factory $FieldHistoryEntryCopyWith(FieldHistoryEntry value, $Res Function(FieldHistoryEntry) _then) = _$FieldHistoryEntryCopyWithImpl;
@useResult
$Res call({
 PropertyValue value, String timestamp, String? source
});


$PropertyValueCopyWith<$Res> get value;

}
/// @nodoc
class _$FieldHistoryEntryCopyWithImpl<$Res>
    implements $FieldHistoryEntryCopyWith<$Res> {
  _$FieldHistoryEntryCopyWithImpl(this._self, this._then);

  final FieldHistoryEntry _self;
  final $Res Function(FieldHistoryEntry) _then;

/// Create a copy of FieldHistoryEntry
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? value = null,Object? timestamp = null,Object? source = freezed,}) {
  return _then(_self.copyWith(
value: null == value ? _self.value : value // ignore: cast_nullable_to_non_nullable
as PropertyValue,timestamp: null == timestamp ? _self.timestamp : timestamp // ignore: cast_nullable_to_non_nullable
as String,source: freezed == source ? _self.source : source // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}
/// Create a copy of FieldHistoryEntry
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$PropertyValueCopyWith<$Res> get value {
  
  return $PropertyValueCopyWith<$Res>(_self.value, (value) {
    return _then(_self.copyWith(value: value));
  });
}
}


/// Adds pattern-matching-related methods to [FieldHistoryEntry].
extension FieldHistoryEntryPatterns on FieldHistoryEntry {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _FieldHistoryEntry value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _FieldHistoryEntry() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _FieldHistoryEntry value)  $default,){
final _that = this;
switch (_that) {
case _FieldHistoryEntry():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _FieldHistoryEntry value)?  $default,){
final _that = this;
switch (_that) {
case _FieldHistoryEntry() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( PropertyValue value,  String timestamp,  String? source)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _FieldHistoryEntry() when $default != null:
return $default(_that.value,_that.timestamp,_that.source);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( PropertyValue value,  String timestamp,  String? source)  $default,) {final _that = this;
switch (_that) {
case _FieldHistoryEntry():
return $default(_that.value,_that.timestamp,_that.source);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( PropertyValue value,  String timestamp,  String? source)?  $default,) {final _that = this;
switch (_that) {
case _FieldHistoryEntry() when $default != null:
return $default(_that.value,_that.timestamp,_that.source);case _:
  return null;

}
}

}

/// @nodoc


class _FieldHistoryEntry implements FieldHistoryEntry {
  const _FieldHistoryEntry({required this.value, required this.timestamp, this.source});
  

@override final  PropertyValue value;
@override final  String timestamp;
@override final  String? source;

/// Create a copy of FieldHistoryEntry
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$FieldHistoryEntryCopyWith<_FieldHistoryEntry> get copyWith => __$FieldHistoryEntryCopyWithImpl<_FieldHistoryEntry>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _FieldHistoryEntry&&(identical(other.value, value) || other.value == value)&&(identical(other.timestamp, timestamp) || other.timestamp == timestamp)&&(identical(other.source, source) || other.source == source));
}


@override
int get hashCode => Object.hash(runtimeType,value,timestamp,source);

@override
String toString() {
  return 'FieldHistoryEntry(value: $value, timestamp: $timestamp, source: $source)';
}


}

/// @nodoc
abstract mixin class _$FieldHistoryEntryCopyWith<$Res> implements $FieldHistoryEntryCopyWith<$Res> {
  factory _$FieldHistoryEntryCopyWith(_FieldHistoryEntry value, $Res Function(_FieldHistoryEntry) _then) = __$FieldHistoryEntryCopyWithImpl;
@override @useResult
$Res call({
 PropertyValue value, String timestamp, String? source
});


@override $PropertyValueCopyWith<$Res> get value;

}
/// @nodoc
class __$FieldHistoryEntryCopyWithImpl<$Res>
    implements _$FieldHistoryEntryCopyWith<$Res> {
  __$FieldHistoryEntryCopyWithImpl(this._self, this._then);

  final _FieldHistoryEntry _self;
  final $Res Function(_FieldHistoryEntry) _then;

/// Create a copy of FieldHistoryEntry
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? value = null,Object? timestamp = null,Object? source = freezed,}) {
  return _then(_FieldHistoryEntry(
value: null == value ? _self.value : value // ignore: cast_nullable_to_non_nullable
as PropertyValue,timestamp: null == timestamp ? _self.timestamp : timestamp // ignore: cast_nullable_to_non_nullable
as String,source: freezed == source ? _self.source : source // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}

/// Create a copy of FieldHistoryEntry
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$PropertyValueCopyWith<$Res> get value {
  
  return $PropertyValueCopyWith<$Res>(_self.value, (value) {
    return _then(_self.copyWith(value: value));
  });
}
}

/// @nodoc
mixin _$FormHistories {

 Map<String, Map<String, List<FieldHistoryEntry>>> get histories;
/// Create a copy of FormHistories
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FormHistoriesCopyWith<FormHistories> get copyWith => _$FormHistoriesCopyWithImpl<FormHistories>(this as FormHistories, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FormHistories&&const DeepCollectionEquality().equals(other.histories, histories));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(histories));

@override
String toString() {
  return 'FormHistories(histories: $histories)';
}


}

/// @nodoc
abstract mixin class $FormHistoriesCopyWith<$Res>  {
  factory $FormHistoriesCopyWith(FormHistories value, $Res Function(FormHistories) _then) = _$FormHistoriesCopyWithImpl;
@useResult
$Res call({
 Map<String, Map<String, List<FieldHistoryEntry>>> histories
});




}
/// @nodoc
class _$FormHistoriesCopyWithImpl<$Res>
    implements $FormHistoriesCopyWith<$Res> {
  _$FormHistoriesCopyWithImpl(this._self, this._then);

  final FormHistories _self;
  final $Res Function(FormHistories) _then;

/// Create a copy of FormHistories
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? histories = null,}) {
  return _then(_self.copyWith(
histories: null == histories ? _self.histories : histories // ignore: cast_nullable_to_non_nullable
as Map<String, Map<String, List<FieldHistoryEntry>>>,
  ));
}

}


/// Adds pattern-matching-related methods to [FormHistories].
extension FormHistoriesPatterns on FormHistories {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _FormHistories value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _FormHistories() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _FormHistories value)  $default,){
final _that = this;
switch (_that) {
case _FormHistories():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _FormHistories value)?  $default,){
final _that = this;
switch (_that) {
case _FormHistories() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( Map<String, Map<String, List<FieldHistoryEntry>>> histories)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _FormHistories() when $default != null:
return $default(_that.histories);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( Map<String, Map<String, List<FieldHistoryEntry>>> histories)  $default,) {final _that = this;
switch (_that) {
case _FormHistories():
return $default(_that.histories);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( Map<String, Map<String, List<FieldHistoryEntry>>> histories)?  $default,) {final _that = this;
switch (_that) {
case _FormHistories() when $default != null:
return $default(_that.histories);case _:
  return null;

}
}

}

/// @nodoc


class _FormHistories implements FormHistories {
  const _FormHistories({required final  Map<String, Map<String, List<FieldHistoryEntry>>> histories}): _histories = histories;
  

 final  Map<String, Map<String, List<FieldHistoryEntry>>> _histories;
@override Map<String, Map<String, List<FieldHistoryEntry>>> get histories {
  if (_histories is EqualUnmodifiableMapView) return _histories;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableMapView(_histories);
}


/// Create a copy of FormHistories
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$FormHistoriesCopyWith<_FormHistories> get copyWith => __$FormHistoriesCopyWithImpl<_FormHistories>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _FormHistories&&const DeepCollectionEquality().equals(other._histories, _histories));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_histories));

@override
String toString() {
  return 'FormHistories(histories: $histories)';
}


}

/// @nodoc
abstract mixin class _$FormHistoriesCopyWith<$Res> implements $FormHistoriesCopyWith<$Res> {
  factory _$FormHistoriesCopyWith(_FormHistories value, $Res Function(_FormHistories) _then) = __$FormHistoriesCopyWithImpl;
@override @useResult
$Res call({
 Map<String, Map<String, List<FieldHistoryEntry>>> histories
});




}
/// @nodoc
class __$FormHistoriesCopyWithImpl<$Res>
    implements _$FormHistoriesCopyWith<$Res> {
  __$FormHistoriesCopyWithImpl(this._self, this._then);

  final _FormHistories _self;
  final $Res Function(_FormHistories) _then;

/// Create a copy of FormHistories
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? histories = null,}) {
  return _then(_FormHistories(
histories: null == histories ? _self._histories : histories // ignore: cast_nullable_to_non_nullable
as Map<String, Map<String, List<FieldHistoryEntry>>>,
  ));
}


}

/// @nodoc
mixin _$LoadedProfile {

 String get id; String get name; Uint8List get data; int get version;
/// Create a copy of LoadedProfile
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LoadedProfileCopyWith<LoadedProfile> get copyWith => _$LoadedProfileCopyWithImpl<LoadedProfile>(this as LoadedProfile, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LoadedProfile&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name)&&const DeepCollectionEquality().equals(other.data, data)&&(identical(other.version, version) || other.version == version));
}


@override
int get hashCode => Object.hash(runtimeType,id,name,const DeepCollectionEquality().hash(data),version);

@override
String toString() {
  return 'LoadedProfile(id: $id, name: $name, data: $data, version: $version)';
}


}

/// @nodoc
abstract mixin class $LoadedProfileCopyWith<$Res>  {
  factory $LoadedProfileCopyWith(LoadedProfile value, $Res Function(LoadedProfile) _then) = _$LoadedProfileCopyWithImpl;
@useResult
$Res call({
 String id, String name, Uint8List data, int version
});




}
/// @nodoc
class _$LoadedProfileCopyWithImpl<$Res>
    implements $LoadedProfileCopyWith<$Res> {
  _$LoadedProfileCopyWithImpl(this._self, this._then);

  final LoadedProfile _self;
  final $Res Function(LoadedProfile) _then;

/// Create a copy of LoadedProfile
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? id = null,Object? name = null,Object? data = null,Object? version = null,}) {
  return _then(_self.copyWith(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,data: null == data ? _self.data : data // ignore: cast_nullable_to_non_nullable
as Uint8List,version: null == version ? _self.version : version // ignore: cast_nullable_to_non_nullable
as int,
  ));
}

}


/// Adds pattern-matching-related methods to [LoadedProfile].
extension LoadedProfilePatterns on LoadedProfile {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _LoadedProfile value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _LoadedProfile() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _LoadedProfile value)  $default,){
final _that = this;
switch (_that) {
case _LoadedProfile():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _LoadedProfile value)?  $default,){
final _that = this;
switch (_that) {
case _LoadedProfile() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String id,  String name,  Uint8List data,  int version)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _LoadedProfile() when $default != null:
return $default(_that.id,_that.name,_that.data,_that.version);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String id,  String name,  Uint8List data,  int version)  $default,) {final _that = this;
switch (_that) {
case _LoadedProfile():
return $default(_that.id,_that.name,_that.data,_that.version);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String id,  String name,  Uint8List data,  int version)?  $default,) {final _that = this;
switch (_that) {
case _LoadedProfile() when $default != null:
return $default(_that.id,_that.name,_that.data,_that.version);case _:
  return null;

}
}

}

/// @nodoc


class _LoadedProfile implements LoadedProfile {
  const _LoadedProfile({required this.id, required this.name, required this.data, required this.version});
  

@override final  String id;
@override final  String name;
@override final  Uint8List data;
@override final  int version;

/// Create a copy of LoadedProfile
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$LoadedProfileCopyWith<_LoadedProfile> get copyWith => __$LoadedProfileCopyWithImpl<_LoadedProfile>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _LoadedProfile&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name)&&const DeepCollectionEquality().equals(other.data, data)&&(identical(other.version, version) || other.version == version));
}


@override
int get hashCode => Object.hash(runtimeType,id,name,const DeepCollectionEquality().hash(data),version);

@override
String toString() {
  return 'LoadedProfile(id: $id, name: $name, data: $data, version: $version)';
}


}

/// @nodoc
abstract mixin class _$LoadedProfileCopyWith<$Res> implements $LoadedProfileCopyWith<$Res> {
  factory _$LoadedProfileCopyWith(_LoadedProfile value, $Res Function(_LoadedProfile) _then) = __$LoadedProfileCopyWithImpl;
@override @useResult
$Res call({
 String id, String name, Uint8List data, int version
});




}
/// @nodoc
class __$LoadedProfileCopyWithImpl<$Res>
    implements _$LoadedProfileCopyWith<$Res> {
  __$LoadedProfileCopyWithImpl(this._self, this._then);

  final _LoadedProfile _self;
  final $Res Function(_LoadedProfile) _then;

/// Create a copy of LoadedProfile
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? id = null,Object? name = null,Object? data = null,Object? version = null,}) {
  return _then(_LoadedProfile(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,data: null == data ? _self.data : data // ignore: cast_nullable_to_non_nullable
as Uint8List,version: null == version ? _self.version : version // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc
mixin _$ProfileSummary {

 String get id; String get name; String get createdAt; String get updatedAt; int get version;
/// Create a copy of ProfileSummary
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ProfileSummaryCopyWith<ProfileSummary> get copyWith => _$ProfileSummaryCopyWithImpl<ProfileSummary>(this as ProfileSummary, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ProfileSummary&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name)&&(identical(other.createdAt, createdAt) || other.createdAt == createdAt)&&(identical(other.updatedAt, updatedAt) || other.updatedAt == updatedAt)&&(identical(other.version, version) || other.version == version));
}


@override
int get hashCode => Object.hash(runtimeType,id,name,createdAt,updatedAt,version);

@override
String toString() {
  return 'ProfileSummary(id: $id, name: $name, createdAt: $createdAt, updatedAt: $updatedAt, version: $version)';
}


}

/// @nodoc
abstract mixin class $ProfileSummaryCopyWith<$Res>  {
  factory $ProfileSummaryCopyWith(ProfileSummary value, $Res Function(ProfileSummary) _then) = _$ProfileSummaryCopyWithImpl;
@useResult
$Res call({
 String id, String name, String createdAt, String updatedAt, int version
});




}
/// @nodoc
class _$ProfileSummaryCopyWithImpl<$Res>
    implements $ProfileSummaryCopyWith<$Res> {
  _$ProfileSummaryCopyWithImpl(this._self, this._then);

  final ProfileSummary _self;
  final $Res Function(ProfileSummary) _then;

/// Create a copy of ProfileSummary
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? id = null,Object? name = null,Object? createdAt = null,Object? updatedAt = null,Object? version = null,}) {
  return _then(_self.copyWith(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,createdAt: null == createdAt ? _self.createdAt : createdAt // ignore: cast_nullable_to_non_nullable
as String,updatedAt: null == updatedAt ? _self.updatedAt : updatedAt // ignore: cast_nullable_to_non_nullable
as String,version: null == version ? _self.version : version // ignore: cast_nullable_to_non_nullable
as int,
  ));
}

}


/// Adds pattern-matching-related methods to [ProfileSummary].
extension ProfileSummaryPatterns on ProfileSummary {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ProfileSummary value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ProfileSummary() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ProfileSummary value)  $default,){
final _that = this;
switch (_that) {
case _ProfileSummary():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ProfileSummary value)?  $default,){
final _that = this;
switch (_that) {
case _ProfileSummary() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String id,  String name,  String createdAt,  String updatedAt,  int version)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ProfileSummary() when $default != null:
return $default(_that.id,_that.name,_that.createdAt,_that.updatedAt,_that.version);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String id,  String name,  String createdAt,  String updatedAt,  int version)  $default,) {final _that = this;
switch (_that) {
case _ProfileSummary():
return $default(_that.id,_that.name,_that.createdAt,_that.updatedAt,_that.version);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String id,  String name,  String createdAt,  String updatedAt,  int version)?  $default,) {final _that = this;
switch (_that) {
case _ProfileSummary() when $default != null:
return $default(_that.id,_that.name,_that.createdAt,_that.updatedAt,_that.version);case _:
  return null;

}
}

}

/// @nodoc


class _ProfileSummary implements ProfileSummary {
  const _ProfileSummary({required this.id, required this.name, required this.createdAt, required this.updatedAt, required this.version});
  

@override final  String id;
@override final  String name;
@override final  String createdAt;
@override final  String updatedAt;
@override final  int version;

/// Create a copy of ProfileSummary
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ProfileSummaryCopyWith<_ProfileSummary> get copyWith => __$ProfileSummaryCopyWithImpl<_ProfileSummary>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ProfileSummary&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name)&&(identical(other.createdAt, createdAt) || other.createdAt == createdAt)&&(identical(other.updatedAt, updatedAt) || other.updatedAt == updatedAt)&&(identical(other.version, version) || other.version == version));
}


@override
int get hashCode => Object.hash(runtimeType,id,name,createdAt,updatedAt,version);

@override
String toString() {
  return 'ProfileSummary(id: $id, name: $name, createdAt: $createdAt, updatedAt: $updatedAt, version: $version)';
}


}

/// @nodoc
abstract mixin class _$ProfileSummaryCopyWith<$Res> implements $ProfileSummaryCopyWith<$Res> {
  factory _$ProfileSummaryCopyWith(_ProfileSummary value, $Res Function(_ProfileSummary) _then) = __$ProfileSummaryCopyWithImpl;
@override @useResult
$Res call({
 String id, String name, String createdAt, String updatedAt, int version
});




}
/// @nodoc
class __$ProfileSummaryCopyWithImpl<$Res>
    implements _$ProfileSummaryCopyWith<$Res> {
  __$ProfileSummaryCopyWithImpl(this._self, this._then);

  final _ProfileSummary _self;
  final $Res Function(_ProfileSummary) _then;

/// Create a copy of ProfileSummary
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? id = null,Object? name = null,Object? createdAt = null,Object? updatedAt = null,Object? version = null,}) {
  return _then(_ProfileSummary(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,createdAt: null == createdAt ? _self.createdAt : createdAt // ignore: cast_nullable_to_non_nullable
as String,updatedAt: null == updatedAt ? _self.updatedAt : updatedAt // ignore: cast_nullable_to_non_nullable
as String,version: null == version ? _self.version : version // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc
mixin _$PropertyValue {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PropertyValue);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'PropertyValue()';
}


}

/// @nodoc
class $PropertyValueCopyWith<$Res>  {
$PropertyValueCopyWith(PropertyValue _, $Res Function(PropertyValue) __);
}


/// Adds pattern-matching-related methods to [PropertyValue].
extension PropertyValuePatterns on PropertyValue {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( PropertyValue_Text value)?  text,TResult Function( PropertyValue_Number value)?  number,TResult Function( PropertyValue_Boolean value)?  boolean,TResult Function( PropertyValue_RichText value)?  richText,required TResult orElse(),}){
final _that = this;
switch (_that) {
case PropertyValue_Text() when text != null:
return text(_that);case PropertyValue_Number() when number != null:
return number(_that);case PropertyValue_Boolean() when boolean != null:
return boolean(_that);case PropertyValue_RichText() when richText != null:
return richText(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( PropertyValue_Text value)  text,required TResult Function( PropertyValue_Number value)  number,required TResult Function( PropertyValue_Boolean value)  boolean,required TResult Function( PropertyValue_RichText value)  richText,}){
final _that = this;
switch (_that) {
case PropertyValue_Text():
return text(_that);case PropertyValue_Number():
return number(_that);case PropertyValue_Boolean():
return boolean(_that);case PropertyValue_RichText():
return richText(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( PropertyValue_Text value)?  text,TResult? Function( PropertyValue_Number value)?  number,TResult? Function( PropertyValue_Boolean value)?  boolean,TResult? Function( PropertyValue_RichText value)?  richText,}){
final _that = this;
switch (_that) {
case PropertyValue_Text() when text != null:
return text(_that);case PropertyValue_Number() when number != null:
return number(_that);case PropertyValue_Boolean() when boolean != null:
return boolean(_that);case PropertyValue_RichText() when richText != null:
return richText(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String text,  SensitivityLevel sensitivity)?  text,TResult Function( double value)?  number,TResult Function( bool value)?  boolean,TResult Function( String html,  SensitivityLevel sensitivity)?  richText,required TResult orElse(),}) {final _that = this;
switch (_that) {
case PropertyValue_Text() when text != null:
return text(_that.text,_that.sensitivity);case PropertyValue_Number() when number != null:
return number(_that.value);case PropertyValue_Boolean() when boolean != null:
return boolean(_that.value);case PropertyValue_RichText() when richText != null:
return richText(_that.html,_that.sensitivity);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String text,  SensitivityLevel sensitivity)  text,required TResult Function( double value)  number,required TResult Function( bool value)  boolean,required TResult Function( String html,  SensitivityLevel sensitivity)  richText,}) {final _that = this;
switch (_that) {
case PropertyValue_Text():
return text(_that.text,_that.sensitivity);case PropertyValue_Number():
return number(_that.value);case PropertyValue_Boolean():
return boolean(_that.value);case PropertyValue_RichText():
return richText(_that.html,_that.sensitivity);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String text,  SensitivityLevel sensitivity)?  text,TResult? Function( double value)?  number,TResult? Function( bool value)?  boolean,TResult? Function( String html,  SensitivityLevel sensitivity)?  richText,}) {final _that = this;
switch (_that) {
case PropertyValue_Text() when text != null:
return text(_that.text,_that.sensitivity);case PropertyValue_Number() when number != null:
return number(_that.value);case PropertyValue_Boolean() when boolean != null:
return boolean(_that.value);case PropertyValue_RichText() when richText != null:
return richText(_that.html,_that.sensitivity);case _:
  return null;

}
}

}

/// @nodoc


class PropertyValue_Text extends PropertyValue {
  const PropertyValue_Text({required this.text, required this.sensitivity}): super._();
  

 final  String text;
 final  SensitivityLevel sensitivity;

/// Create a copy of PropertyValue
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PropertyValue_TextCopyWith<PropertyValue_Text> get copyWith => _$PropertyValue_TextCopyWithImpl<PropertyValue_Text>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PropertyValue_Text&&(identical(other.text, text) || other.text == text)&&(identical(other.sensitivity, sensitivity) || other.sensitivity == sensitivity));
}


@override
int get hashCode => Object.hash(runtimeType,text,sensitivity);

@override
String toString() {
  return 'PropertyValue.text(text: $text, sensitivity: $sensitivity)';
}


}

/// @nodoc
abstract mixin class $PropertyValue_TextCopyWith<$Res> implements $PropertyValueCopyWith<$Res> {
  factory $PropertyValue_TextCopyWith(PropertyValue_Text value, $Res Function(PropertyValue_Text) _then) = _$PropertyValue_TextCopyWithImpl;
@useResult
$Res call({
 String text, SensitivityLevel sensitivity
});




}
/// @nodoc
class _$PropertyValue_TextCopyWithImpl<$Res>
    implements $PropertyValue_TextCopyWith<$Res> {
  _$PropertyValue_TextCopyWithImpl(this._self, this._then);

  final PropertyValue_Text _self;
  final $Res Function(PropertyValue_Text) _then;

/// Create a copy of PropertyValue
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? text = null,Object? sensitivity = null,}) {
  return _then(PropertyValue_Text(
text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,sensitivity: null == sensitivity ? _self.sensitivity : sensitivity // ignore: cast_nullable_to_non_nullable
as SensitivityLevel,
  ));
}


}

/// @nodoc


class PropertyValue_Number extends PropertyValue {
  const PropertyValue_Number({required this.value}): super._();
  

 final  double value;

/// Create a copy of PropertyValue
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PropertyValue_NumberCopyWith<PropertyValue_Number> get copyWith => _$PropertyValue_NumberCopyWithImpl<PropertyValue_Number>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PropertyValue_Number&&(identical(other.value, value) || other.value == value));
}


@override
int get hashCode => Object.hash(runtimeType,value);

@override
String toString() {
  return 'PropertyValue.number(value: $value)';
}


}

/// @nodoc
abstract mixin class $PropertyValue_NumberCopyWith<$Res> implements $PropertyValueCopyWith<$Res> {
  factory $PropertyValue_NumberCopyWith(PropertyValue_Number value, $Res Function(PropertyValue_Number) _then) = _$PropertyValue_NumberCopyWithImpl;
@useResult
$Res call({
 double value
});




}
/// @nodoc
class _$PropertyValue_NumberCopyWithImpl<$Res>
    implements $PropertyValue_NumberCopyWith<$Res> {
  _$PropertyValue_NumberCopyWithImpl(this._self, this._then);

  final PropertyValue_Number _self;
  final $Res Function(PropertyValue_Number) _then;

/// Create a copy of PropertyValue
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? value = null,}) {
  return _then(PropertyValue_Number(
value: null == value ? _self.value : value // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc


class PropertyValue_Boolean extends PropertyValue {
  const PropertyValue_Boolean({required this.value}): super._();
  

 final  bool value;

/// Create a copy of PropertyValue
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PropertyValue_BooleanCopyWith<PropertyValue_Boolean> get copyWith => _$PropertyValue_BooleanCopyWithImpl<PropertyValue_Boolean>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PropertyValue_Boolean&&(identical(other.value, value) || other.value == value));
}


@override
int get hashCode => Object.hash(runtimeType,value);

@override
String toString() {
  return 'PropertyValue.boolean(value: $value)';
}


}

/// @nodoc
abstract mixin class $PropertyValue_BooleanCopyWith<$Res> implements $PropertyValueCopyWith<$Res> {
  factory $PropertyValue_BooleanCopyWith(PropertyValue_Boolean value, $Res Function(PropertyValue_Boolean) _then) = _$PropertyValue_BooleanCopyWithImpl;
@useResult
$Res call({
 bool value
});




}
/// @nodoc
class _$PropertyValue_BooleanCopyWithImpl<$Res>
    implements $PropertyValue_BooleanCopyWith<$Res> {
  _$PropertyValue_BooleanCopyWithImpl(this._self, this._then);

  final PropertyValue_Boolean _self;
  final $Res Function(PropertyValue_Boolean) _then;

/// Create a copy of PropertyValue
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? value = null,}) {
  return _then(PropertyValue_Boolean(
value: null == value ? _self.value : value // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class PropertyValue_RichText extends PropertyValue {
  const PropertyValue_RichText({required this.html, required this.sensitivity}): super._();
  

 final  String html;
 final  SensitivityLevel sensitivity;

/// Create a copy of PropertyValue
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PropertyValue_RichTextCopyWith<PropertyValue_RichText> get copyWith => _$PropertyValue_RichTextCopyWithImpl<PropertyValue_RichText>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PropertyValue_RichText&&(identical(other.html, html) || other.html == html)&&(identical(other.sensitivity, sensitivity) || other.sensitivity == sensitivity));
}


@override
int get hashCode => Object.hash(runtimeType,html,sensitivity);

@override
String toString() {
  return 'PropertyValue.richText(html: $html, sensitivity: $sensitivity)';
}


}

/// @nodoc
abstract mixin class $PropertyValue_RichTextCopyWith<$Res> implements $PropertyValueCopyWith<$Res> {
  factory $PropertyValue_RichTextCopyWith(PropertyValue_RichText value, $Res Function(PropertyValue_RichText) _then) = _$PropertyValue_RichTextCopyWithImpl;
@useResult
$Res call({
 String html, SensitivityLevel sensitivity
});




}
/// @nodoc
class _$PropertyValue_RichTextCopyWithImpl<$Res>
    implements $PropertyValue_RichTextCopyWith<$Res> {
  _$PropertyValue_RichTextCopyWithImpl(this._self, this._then);

  final PropertyValue_RichText _self;
  final $Res Function(PropertyValue_RichText) _then;

/// Create a copy of PropertyValue
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? html = null,Object? sensitivity = null,}) {
  return _then(PropertyValue_RichText(
html: null == html ? _self.html : html // ignore: cast_nullable_to_non_nullable
as String,sensitivity: null == sensitivity ? _self.sensitivity : sensitivity // ignore: cast_nullable_to_non_nullable
as SensitivityLevel,
  ));
}


}

/// @nodoc
mixin _$UnlockVaultResult {

 bool get success; String? get error; int? get cryptoVersion;
/// Create a copy of UnlockVaultResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UnlockVaultResultCopyWith<UnlockVaultResult> get copyWith => _$UnlockVaultResultCopyWithImpl<UnlockVaultResult>(this as UnlockVaultResult, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UnlockVaultResult&&(identical(other.success, success) || other.success == success)&&(identical(other.error, error) || other.error == error)&&(identical(other.cryptoVersion, cryptoVersion) || other.cryptoVersion == cryptoVersion));
}


@override
int get hashCode => Object.hash(runtimeType,success,error,cryptoVersion);

@override
String toString() {
  return 'UnlockVaultResult(success: $success, error: $error, cryptoVersion: $cryptoVersion)';
}


}

/// @nodoc
abstract mixin class $UnlockVaultResultCopyWith<$Res>  {
  factory $UnlockVaultResultCopyWith(UnlockVaultResult value, $Res Function(UnlockVaultResult) _then) = _$UnlockVaultResultCopyWithImpl;
@useResult
$Res call({
 bool success, String? error, int? cryptoVersion
});




}
/// @nodoc
class _$UnlockVaultResultCopyWithImpl<$Res>
    implements $UnlockVaultResultCopyWith<$Res> {
  _$UnlockVaultResultCopyWithImpl(this._self, this._then);

  final UnlockVaultResult _self;
  final $Res Function(UnlockVaultResult) _then;

/// Create a copy of UnlockVaultResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? success = null,Object? error = freezed,Object? cryptoVersion = freezed,}) {
  return _then(_self.copyWith(
success: null == success ? _self.success : success // ignore: cast_nullable_to_non_nullable
as bool,error: freezed == error ? _self.error : error // ignore: cast_nullable_to_non_nullable
as String?,cryptoVersion: freezed == cryptoVersion ? _self.cryptoVersion : cryptoVersion // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}

}


/// Adds pattern-matching-related methods to [UnlockVaultResult].
extension UnlockVaultResultPatterns on UnlockVaultResult {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _UnlockVaultResult value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _UnlockVaultResult() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _UnlockVaultResult value)  $default,){
final _that = this;
switch (_that) {
case _UnlockVaultResult():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _UnlockVaultResult value)?  $default,){
final _that = this;
switch (_that) {
case _UnlockVaultResult() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( bool success,  String? error,  int? cryptoVersion)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _UnlockVaultResult() when $default != null:
return $default(_that.success,_that.error,_that.cryptoVersion);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( bool success,  String? error,  int? cryptoVersion)  $default,) {final _that = this;
switch (_that) {
case _UnlockVaultResult():
return $default(_that.success,_that.error,_that.cryptoVersion);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( bool success,  String? error,  int? cryptoVersion)?  $default,) {final _that = this;
switch (_that) {
case _UnlockVaultResult() when $default != null:
return $default(_that.success,_that.error,_that.cryptoVersion);case _:
  return null;

}
}

}

/// @nodoc


class _UnlockVaultResult implements UnlockVaultResult {
  const _UnlockVaultResult({required this.success, this.error, this.cryptoVersion});
  

@override final  bool success;
@override final  String? error;
@override final  int? cryptoVersion;

/// Create a copy of UnlockVaultResult
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$UnlockVaultResultCopyWith<_UnlockVaultResult> get copyWith => __$UnlockVaultResultCopyWithImpl<_UnlockVaultResult>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _UnlockVaultResult&&(identical(other.success, success) || other.success == success)&&(identical(other.error, error) || other.error == error)&&(identical(other.cryptoVersion, cryptoVersion) || other.cryptoVersion == cryptoVersion));
}


@override
int get hashCode => Object.hash(runtimeType,success,error,cryptoVersion);

@override
String toString() {
  return 'UnlockVaultResult(success: $success, error: $error, cryptoVersion: $cryptoVersion)';
}


}

/// @nodoc
abstract mixin class _$UnlockVaultResultCopyWith<$Res> implements $UnlockVaultResultCopyWith<$Res> {
  factory _$UnlockVaultResultCopyWith(_UnlockVaultResult value, $Res Function(_UnlockVaultResult) _then) = __$UnlockVaultResultCopyWithImpl;
@override @useResult
$Res call({
 bool success, String? error, int? cryptoVersion
});




}
/// @nodoc
class __$UnlockVaultResultCopyWithImpl<$Res>
    implements _$UnlockVaultResultCopyWith<$Res> {
  __$UnlockVaultResultCopyWithImpl(this._self, this._then);

  final _UnlockVaultResult _self;
  final $Res Function(_UnlockVaultResult) _then;

/// Create a copy of UnlockVaultResult
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? success = null,Object? error = freezed,Object? cryptoVersion = freezed,}) {
  return _then(_UnlockVaultResult(
success: null == success ? _self.success : success // ignore: cast_nullable_to_non_nullable
as bool,error: freezed == error ? _self.error : error // ignore: cast_nullable_to_non_nullable
as String?,cryptoVersion: freezed == cryptoVersion ? _self.cryptoVersion : cryptoVersion // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}


}

/// @nodoc
mixin _$VaultStats {

 BigInt get profileCount; BigInt get totalSizeBytes; String? get lastModified; String? get accountId;
/// Create a copy of VaultStats
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$VaultStatsCopyWith<VaultStats> get copyWith => _$VaultStatsCopyWithImpl<VaultStats>(this as VaultStats, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VaultStats&&(identical(other.profileCount, profileCount) || other.profileCount == profileCount)&&(identical(other.totalSizeBytes, totalSizeBytes) || other.totalSizeBytes == totalSizeBytes)&&(identical(other.lastModified, lastModified) || other.lastModified == lastModified)&&(identical(other.accountId, accountId) || other.accountId == accountId));
}


@override
int get hashCode => Object.hash(runtimeType,profileCount,totalSizeBytes,lastModified,accountId);

@override
String toString() {
  return 'VaultStats(profileCount: $profileCount, totalSizeBytes: $totalSizeBytes, lastModified: $lastModified, accountId: $accountId)';
}


}

/// @nodoc
abstract mixin class $VaultStatsCopyWith<$Res>  {
  factory $VaultStatsCopyWith(VaultStats value, $Res Function(VaultStats) _then) = _$VaultStatsCopyWithImpl;
@useResult
$Res call({
 BigInt profileCount, BigInt totalSizeBytes, String? lastModified, String? accountId
});




}
/// @nodoc
class _$VaultStatsCopyWithImpl<$Res>
    implements $VaultStatsCopyWith<$Res> {
  _$VaultStatsCopyWithImpl(this._self, this._then);

  final VaultStats _self;
  final $Res Function(VaultStats) _then;

/// Create a copy of VaultStats
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? profileCount = null,Object? totalSizeBytes = null,Object? lastModified = freezed,Object? accountId = freezed,}) {
  return _then(_self.copyWith(
profileCount: null == profileCount ? _self.profileCount : profileCount // ignore: cast_nullable_to_non_nullable
as BigInt,totalSizeBytes: null == totalSizeBytes ? _self.totalSizeBytes : totalSizeBytes // ignore: cast_nullable_to_non_nullable
as BigInt,lastModified: freezed == lastModified ? _self.lastModified : lastModified // ignore: cast_nullable_to_non_nullable
as String?,accountId: freezed == accountId ? _self.accountId : accountId // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}

}


/// Adds pattern-matching-related methods to [VaultStats].
extension VaultStatsPatterns on VaultStats {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _VaultStats value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _VaultStats() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _VaultStats value)  $default,){
final _that = this;
switch (_that) {
case _VaultStats():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _VaultStats value)?  $default,){
final _that = this;
switch (_that) {
case _VaultStats() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( BigInt profileCount,  BigInt totalSizeBytes,  String? lastModified,  String? accountId)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _VaultStats() when $default != null:
return $default(_that.profileCount,_that.totalSizeBytes,_that.lastModified,_that.accountId);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( BigInt profileCount,  BigInt totalSizeBytes,  String? lastModified,  String? accountId)  $default,) {final _that = this;
switch (_that) {
case _VaultStats():
return $default(_that.profileCount,_that.totalSizeBytes,_that.lastModified,_that.accountId);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( BigInt profileCount,  BigInt totalSizeBytes,  String? lastModified,  String? accountId)?  $default,) {final _that = this;
switch (_that) {
case _VaultStats() when $default != null:
return $default(_that.profileCount,_that.totalSizeBytes,_that.lastModified,_that.accountId);case _:
  return null;

}
}

}

/// @nodoc


class _VaultStats implements VaultStats {
  const _VaultStats({required this.profileCount, required this.totalSizeBytes, this.lastModified, this.accountId});
  

@override final  BigInt profileCount;
@override final  BigInt totalSizeBytes;
@override final  String? lastModified;
@override final  String? accountId;

/// Create a copy of VaultStats
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$VaultStatsCopyWith<_VaultStats> get copyWith => __$VaultStatsCopyWithImpl<_VaultStats>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _VaultStats&&(identical(other.profileCount, profileCount) || other.profileCount == profileCount)&&(identical(other.totalSizeBytes, totalSizeBytes) || other.totalSizeBytes == totalSizeBytes)&&(identical(other.lastModified, lastModified) || other.lastModified == lastModified)&&(identical(other.accountId, accountId) || other.accountId == accountId));
}


@override
int get hashCode => Object.hash(runtimeType,profileCount,totalSizeBytes,lastModified,accountId);

@override
String toString() {
  return 'VaultStats(profileCount: $profileCount, totalSizeBytes: $totalSizeBytes, lastModified: $lastModified, accountId: $accountId)';
}


}

/// @nodoc
abstract mixin class _$VaultStatsCopyWith<$Res> implements $VaultStatsCopyWith<$Res> {
  factory _$VaultStatsCopyWith(_VaultStats value, $Res Function(_VaultStats) _then) = __$VaultStatsCopyWithImpl;
@override @useResult
$Res call({
 BigInt profileCount, BigInt totalSizeBytes, String? lastModified, String? accountId
});




}
/// @nodoc
class __$VaultStatsCopyWithImpl<$Res>
    implements _$VaultStatsCopyWith<$Res> {
  __$VaultStatsCopyWithImpl(this._self, this._then);

  final _VaultStats _self;
  final $Res Function(_VaultStats) _then;

/// Create a copy of VaultStats
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? profileCount = null,Object? totalSizeBytes = null,Object? lastModified = freezed,Object? accountId = freezed,}) {
  return _then(_VaultStats(
profileCount: null == profileCount ? _self.profileCount : profileCount // ignore: cast_nullable_to_non_nullable
as BigInt,totalSizeBytes: null == totalSizeBytes ? _self.totalSizeBytes : totalSizeBytes // ignore: cast_nullable_to_non_nullable
as BigInt,lastModified: freezed == lastModified ? _self.lastModified : lastModified // ignore: cast_nullable_to_non_nullable
as String?,accountId: freezed == accountId ? _self.accountId : accountId // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

// dart format on
