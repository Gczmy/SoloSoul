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

 String get id; String get name; String? get createdAt; String? get lastAccessed; String? get passwordHint; String? get lastLoginAt; String? get lastOperationAt; String? get lastOperationDesc;
/// Create a copy of AccountInfo
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AccountInfoCopyWith<AccountInfo> get copyWith => _$AccountInfoCopyWithImpl<AccountInfo>(this as AccountInfo, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AccountInfo&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name)&&(identical(other.createdAt, createdAt) || other.createdAt == createdAt)&&(identical(other.lastAccessed, lastAccessed) || other.lastAccessed == lastAccessed)&&(identical(other.passwordHint, passwordHint) || other.passwordHint == passwordHint)&&(identical(other.lastLoginAt, lastLoginAt) || other.lastLoginAt == lastLoginAt)&&(identical(other.lastOperationAt, lastOperationAt) || other.lastOperationAt == lastOperationAt)&&(identical(other.lastOperationDesc, lastOperationDesc) || other.lastOperationDesc == lastOperationDesc));
}


@override
int get hashCode => Object.hash(runtimeType,id,name,createdAt,lastAccessed,passwordHint,lastLoginAt,lastOperationAt,lastOperationDesc);

@override
String toString() {
  return 'AccountInfo(id: $id, name: $name, createdAt: $createdAt, lastAccessed: $lastAccessed, passwordHint: $passwordHint, lastLoginAt: $lastLoginAt, lastOperationAt: $lastOperationAt, lastOperationDesc: $lastOperationDesc)';
}


}

/// @nodoc
abstract mixin class $AccountInfoCopyWith<$Res>  {
  factory $AccountInfoCopyWith(AccountInfo value, $Res Function(AccountInfo) _then) = _$AccountInfoCopyWithImpl;
@useResult
$Res call({
 String id, String name, String? createdAt, String? lastAccessed, String? passwordHint, String? lastLoginAt, String? lastOperationAt, String? lastOperationDesc
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
@pragma('vm:prefer-inline') @override $Res call({Object? id = null,Object? name = null,Object? createdAt = freezed,Object? lastAccessed = freezed,Object? passwordHint = freezed,Object? lastLoginAt = freezed,Object? lastOperationAt = freezed,Object? lastOperationDesc = freezed,}) {
  return _then(_self.copyWith(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,createdAt: freezed == createdAt ? _self.createdAt : createdAt // ignore: cast_nullable_to_non_nullable
as String?,lastAccessed: freezed == lastAccessed ? _self.lastAccessed : lastAccessed // ignore: cast_nullable_to_non_nullable
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String id,  String name,  String? createdAt,  String? lastAccessed,  String? passwordHint,  String? lastLoginAt,  String? lastOperationAt,  String? lastOperationDesc)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _AccountInfo() when $default != null:
return $default(_that.id,_that.name,_that.createdAt,_that.lastAccessed,_that.passwordHint,_that.lastLoginAt,_that.lastOperationAt,_that.lastOperationDesc);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String id,  String name,  String? createdAt,  String? lastAccessed,  String? passwordHint,  String? lastLoginAt,  String? lastOperationAt,  String? lastOperationDesc)  $default,) {final _that = this;
switch (_that) {
case _AccountInfo():
return $default(_that.id,_that.name,_that.createdAt,_that.lastAccessed,_that.passwordHint,_that.lastLoginAt,_that.lastOperationAt,_that.lastOperationDesc);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String id,  String name,  String? createdAt,  String? lastAccessed,  String? passwordHint,  String? lastLoginAt,  String? lastOperationAt,  String? lastOperationDesc)?  $default,) {final _that = this;
switch (_that) {
case _AccountInfo() when $default != null:
return $default(_that.id,_that.name,_that.createdAt,_that.lastAccessed,_that.passwordHint,_that.lastLoginAt,_that.lastOperationAt,_that.lastOperationDesc);case _:
  return null;

}
}

}

/// @nodoc


class _AccountInfo implements AccountInfo {
  const _AccountInfo({required this.id, required this.name, this.createdAt, this.lastAccessed, this.passwordHint, this.lastLoginAt, this.lastOperationAt, this.lastOperationDesc});
  

@override final  String id;
@override final  String name;
@override final  String? createdAt;
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
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _AccountInfo&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name)&&(identical(other.createdAt, createdAt) || other.createdAt == createdAt)&&(identical(other.lastAccessed, lastAccessed) || other.lastAccessed == lastAccessed)&&(identical(other.passwordHint, passwordHint) || other.passwordHint == passwordHint)&&(identical(other.lastLoginAt, lastLoginAt) || other.lastLoginAt == lastLoginAt)&&(identical(other.lastOperationAt, lastOperationAt) || other.lastOperationAt == lastOperationAt)&&(identical(other.lastOperationDesc, lastOperationDesc) || other.lastOperationDesc == lastOperationDesc));
}


@override
int get hashCode => Object.hash(runtimeType,id,name,createdAt,lastAccessed,passwordHint,lastLoginAt,lastOperationAt,lastOperationDesc);

@override
String toString() {
  return 'AccountInfo(id: $id, name: $name, createdAt: $createdAt, lastAccessed: $lastAccessed, passwordHint: $passwordHint, lastLoginAt: $lastLoginAt, lastOperationAt: $lastOperationAt, lastOperationDesc: $lastOperationDesc)';
}


}

/// @nodoc
abstract mixin class _$AccountInfoCopyWith<$Res> implements $AccountInfoCopyWith<$Res> {
  factory _$AccountInfoCopyWith(_AccountInfo value, $Res Function(_AccountInfo) _then) = __$AccountInfoCopyWithImpl;
@override @useResult
$Res call({
 String id, String name, String? createdAt, String? lastAccessed, String? passwordHint, String? lastLoginAt, String? lastOperationAt, String? lastOperationDesc
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
@override @pragma('vm:prefer-inline') $Res call({Object? id = null,Object? name = null,Object? createdAt = freezed,Object? lastAccessed = freezed,Object? passwordHint = freezed,Object? lastLoginAt = freezed,Object? lastOperationAt = freezed,Object? lastOperationDesc = freezed,}) {
  return _then(_AccountInfo(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,createdAt: freezed == createdAt ? _self.createdAt : createdAt // ignore: cast_nullable_to_non_nullable
as String?,lastAccessed: freezed == lastAccessed ? _self.lastAccessed : lastAccessed // ignore: cast_nullable_to_non_nullable
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
mixin _$DiscoveredDevice {

 String get name; String get host; int get port; List<String> get addresses;
/// Create a copy of DiscoveredDevice
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$DiscoveredDeviceCopyWith<DiscoveredDevice> get copyWith => _$DiscoveredDeviceCopyWithImpl<DiscoveredDevice>(this as DiscoveredDevice, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is DiscoveredDevice&&(identical(other.name, name) || other.name == name)&&(identical(other.host, host) || other.host == host)&&(identical(other.port, port) || other.port == port)&&const DeepCollectionEquality().equals(other.addresses, addresses));
}


@override
int get hashCode => Object.hash(runtimeType,name,host,port,const DeepCollectionEquality().hash(addresses));

@override
String toString() {
  return 'DiscoveredDevice(name: $name, host: $host, port: $port, addresses: $addresses)';
}


}

/// @nodoc
abstract mixin class $DiscoveredDeviceCopyWith<$Res>  {
  factory $DiscoveredDeviceCopyWith(DiscoveredDevice value, $Res Function(DiscoveredDevice) _then) = _$DiscoveredDeviceCopyWithImpl;
@useResult
$Res call({
 String name, String host, int port, List<String> addresses
});




}
/// @nodoc
class _$DiscoveredDeviceCopyWithImpl<$Res>
    implements $DiscoveredDeviceCopyWith<$Res> {
  _$DiscoveredDeviceCopyWithImpl(this._self, this._then);

  final DiscoveredDevice _self;
  final $Res Function(DiscoveredDevice) _then;

/// Create a copy of DiscoveredDevice
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? name = null,Object? host = null,Object? port = null,Object? addresses = null,}) {
  return _then(_self.copyWith(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,host: null == host ? _self.host : host // ignore: cast_nullable_to_non_nullable
as String,port: null == port ? _self.port : port // ignore: cast_nullable_to_non_nullable
as int,addresses: null == addresses ? _self.addresses : addresses // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}

}


/// Adds pattern-matching-related methods to [DiscoveredDevice].
extension DiscoveredDevicePatterns on DiscoveredDevice {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _DiscoveredDevice value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _DiscoveredDevice() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _DiscoveredDevice value)  $default,){
final _that = this;
switch (_that) {
case _DiscoveredDevice():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _DiscoveredDevice value)?  $default,){
final _that = this;
switch (_that) {
case _DiscoveredDevice() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String name,  String host,  int port,  List<String> addresses)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _DiscoveredDevice() when $default != null:
return $default(_that.name,_that.host,_that.port,_that.addresses);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String name,  String host,  int port,  List<String> addresses)  $default,) {final _that = this;
switch (_that) {
case _DiscoveredDevice():
return $default(_that.name,_that.host,_that.port,_that.addresses);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String name,  String host,  int port,  List<String> addresses)?  $default,) {final _that = this;
switch (_that) {
case _DiscoveredDevice() when $default != null:
return $default(_that.name,_that.host,_that.port,_that.addresses);case _:
  return null;

}
}

}

/// @nodoc


class _DiscoveredDevice implements DiscoveredDevice {
  const _DiscoveredDevice({required this.name, required this.host, required this.port, required final  List<String> addresses}): _addresses = addresses;
  

@override final  String name;
@override final  String host;
@override final  int port;
 final  List<String> _addresses;
@override List<String> get addresses {
  if (_addresses is EqualUnmodifiableListView) return _addresses;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_addresses);
}


/// Create a copy of DiscoveredDevice
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$DiscoveredDeviceCopyWith<_DiscoveredDevice> get copyWith => __$DiscoveredDeviceCopyWithImpl<_DiscoveredDevice>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _DiscoveredDevice&&(identical(other.name, name) || other.name == name)&&(identical(other.host, host) || other.host == host)&&(identical(other.port, port) || other.port == port)&&const DeepCollectionEquality().equals(other._addresses, _addresses));
}


@override
int get hashCode => Object.hash(runtimeType,name,host,port,const DeepCollectionEquality().hash(_addresses));

@override
String toString() {
  return 'DiscoveredDevice(name: $name, host: $host, port: $port, addresses: $addresses)';
}


}

/// @nodoc
abstract mixin class _$DiscoveredDeviceCopyWith<$Res> implements $DiscoveredDeviceCopyWith<$Res> {
  factory _$DiscoveredDeviceCopyWith(_DiscoveredDevice value, $Res Function(_DiscoveredDevice) _then) = __$DiscoveredDeviceCopyWithImpl;
@override @useResult
$Res call({
 String name, String host, int port, List<String> addresses
});




}
/// @nodoc
class __$DiscoveredDeviceCopyWithImpl<$Res>
    implements _$DiscoveredDeviceCopyWith<$Res> {
  __$DiscoveredDeviceCopyWithImpl(this._self, this._then);

  final _DiscoveredDevice _self;
  final $Res Function(_DiscoveredDevice) _then;

/// Create a copy of DiscoveredDevice
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? name = null,Object? host = null,Object? port = null,Object? addresses = null,}) {
  return _then(_DiscoveredDevice(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,host: null == host ? _self.host : host // ignore: cast_nullable_to_non_nullable
as String,port: null == port ? _self.port : port // ignore: cast_nullable_to_non_nullable
as int,addresses: null == addresses ? _self._addresses : addresses // ignore: cast_nullable_to_non_nullable
as List<String>,
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
mixin _$FrbBoundingBox {

 double get x; double get y; double get width; double get height;
/// Create a copy of FrbBoundingBox
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FrbBoundingBoxCopyWith<FrbBoundingBox> get copyWith => _$FrbBoundingBoxCopyWithImpl<FrbBoundingBox>(this as FrbBoundingBox, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FrbBoundingBox&&(identical(other.x, x) || other.x == x)&&(identical(other.y, y) || other.y == y)&&(identical(other.width, width) || other.width == width)&&(identical(other.height, height) || other.height == height));
}


@override
int get hashCode => Object.hash(runtimeType,x,y,width,height);

@override
String toString() {
  return 'FrbBoundingBox(x: $x, y: $y, width: $width, height: $height)';
}


}

/// @nodoc
abstract mixin class $FrbBoundingBoxCopyWith<$Res>  {
  factory $FrbBoundingBoxCopyWith(FrbBoundingBox value, $Res Function(FrbBoundingBox) _then) = _$FrbBoundingBoxCopyWithImpl;
@useResult
$Res call({
 double x, double y, double width, double height
});




}
/// @nodoc
class _$FrbBoundingBoxCopyWithImpl<$Res>
    implements $FrbBoundingBoxCopyWith<$Res> {
  _$FrbBoundingBoxCopyWithImpl(this._self, this._then);

  final FrbBoundingBox _self;
  final $Res Function(FrbBoundingBox) _then;

/// Create a copy of FrbBoundingBox
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? x = null,Object? y = null,Object? width = null,Object? height = null,}) {
  return _then(_self.copyWith(
x: null == x ? _self.x : x // ignore: cast_nullable_to_non_nullable
as double,y: null == y ? _self.y : y // ignore: cast_nullable_to_non_nullable
as double,width: null == width ? _self.width : width // ignore: cast_nullable_to_non_nullable
as double,height: null == height ? _self.height : height // ignore: cast_nullable_to_non_nullable
as double,
  ));
}

}


/// Adds pattern-matching-related methods to [FrbBoundingBox].
extension FrbBoundingBoxPatterns on FrbBoundingBox {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _FrbBoundingBox value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _FrbBoundingBox() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _FrbBoundingBox value)  $default,){
final _that = this;
switch (_that) {
case _FrbBoundingBox():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _FrbBoundingBox value)?  $default,){
final _that = this;
switch (_that) {
case _FrbBoundingBox() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( double x,  double y,  double width,  double height)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _FrbBoundingBox() when $default != null:
return $default(_that.x,_that.y,_that.width,_that.height);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( double x,  double y,  double width,  double height)  $default,) {final _that = this;
switch (_that) {
case _FrbBoundingBox():
return $default(_that.x,_that.y,_that.width,_that.height);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( double x,  double y,  double width,  double height)?  $default,) {final _that = this;
switch (_that) {
case _FrbBoundingBox() when $default != null:
return $default(_that.x,_that.y,_that.width,_that.height);case _:
  return null;

}
}

}

/// @nodoc


class _FrbBoundingBox implements FrbBoundingBox {
  const _FrbBoundingBox({required this.x, required this.y, required this.width, required this.height});
  

@override final  double x;
@override final  double y;
@override final  double width;
@override final  double height;

/// Create a copy of FrbBoundingBox
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$FrbBoundingBoxCopyWith<_FrbBoundingBox> get copyWith => __$FrbBoundingBoxCopyWithImpl<_FrbBoundingBox>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _FrbBoundingBox&&(identical(other.x, x) || other.x == x)&&(identical(other.y, y) || other.y == y)&&(identical(other.width, width) || other.width == width)&&(identical(other.height, height) || other.height == height));
}


@override
int get hashCode => Object.hash(runtimeType,x,y,width,height);

@override
String toString() {
  return 'FrbBoundingBox(x: $x, y: $y, width: $width, height: $height)';
}


}

/// @nodoc
abstract mixin class _$FrbBoundingBoxCopyWith<$Res> implements $FrbBoundingBoxCopyWith<$Res> {
  factory _$FrbBoundingBoxCopyWith(_FrbBoundingBox value, $Res Function(_FrbBoundingBox) _then) = __$FrbBoundingBoxCopyWithImpl;
@override @useResult
$Res call({
 double x, double y, double width, double height
});




}
/// @nodoc
class __$FrbBoundingBoxCopyWithImpl<$Res>
    implements _$FrbBoundingBoxCopyWith<$Res> {
  __$FrbBoundingBoxCopyWithImpl(this._self, this._then);

  final _FrbBoundingBox _self;
  final $Res Function(_FrbBoundingBox) _then;

/// Create a copy of FrbBoundingBox
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? x = null,Object? y = null,Object? width = null,Object? height = null,}) {
  return _then(_FrbBoundingBox(
x: null == x ? _self.x : x // ignore: cast_nullable_to_non_nullable
as double,y: null == y ? _self.y : y // ignore: cast_nullable_to_non_nullable
as double,width: null == width ? _self.width : width // ignore: cast_nullable_to_non_nullable
as double,height: null == height ? _self.height : height // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc
mixin _$FrbOcrBlock {

 String get text; double get confidence; FrbBoundingBox get bbox;
/// Create a copy of FrbOcrBlock
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FrbOcrBlockCopyWith<FrbOcrBlock> get copyWith => _$FrbOcrBlockCopyWithImpl<FrbOcrBlock>(this as FrbOcrBlock, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FrbOcrBlock&&(identical(other.text, text) || other.text == text)&&(identical(other.confidence, confidence) || other.confidence == confidence)&&(identical(other.bbox, bbox) || other.bbox == bbox));
}


@override
int get hashCode => Object.hash(runtimeType,text,confidence,bbox);

@override
String toString() {
  return 'FrbOcrBlock(text: $text, confidence: $confidence, bbox: $bbox)';
}


}

/// @nodoc
abstract mixin class $FrbOcrBlockCopyWith<$Res>  {
  factory $FrbOcrBlockCopyWith(FrbOcrBlock value, $Res Function(FrbOcrBlock) _then) = _$FrbOcrBlockCopyWithImpl;
@useResult
$Res call({
 String text, double confidence, FrbBoundingBox bbox
});


$FrbBoundingBoxCopyWith<$Res> get bbox;

}
/// @nodoc
class _$FrbOcrBlockCopyWithImpl<$Res>
    implements $FrbOcrBlockCopyWith<$Res> {
  _$FrbOcrBlockCopyWithImpl(this._self, this._then);

  final FrbOcrBlock _self;
  final $Res Function(FrbOcrBlock) _then;

/// Create a copy of FrbOcrBlock
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? text = null,Object? confidence = null,Object? bbox = null,}) {
  return _then(_self.copyWith(
text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,confidence: null == confidence ? _self.confidence : confidence // ignore: cast_nullable_to_non_nullable
as double,bbox: null == bbox ? _self.bbox : bbox // ignore: cast_nullable_to_non_nullable
as FrbBoundingBox,
  ));
}
/// Create a copy of FrbOcrBlock
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$FrbBoundingBoxCopyWith<$Res> get bbox {
  
  return $FrbBoundingBoxCopyWith<$Res>(_self.bbox, (value) {
    return _then(_self.copyWith(bbox: value));
  });
}
}


/// Adds pattern-matching-related methods to [FrbOcrBlock].
extension FrbOcrBlockPatterns on FrbOcrBlock {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _FrbOcrBlock value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _FrbOcrBlock() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _FrbOcrBlock value)  $default,){
final _that = this;
switch (_that) {
case _FrbOcrBlock():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _FrbOcrBlock value)?  $default,){
final _that = this;
switch (_that) {
case _FrbOcrBlock() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String text,  double confidence,  FrbBoundingBox bbox)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _FrbOcrBlock() when $default != null:
return $default(_that.text,_that.confidence,_that.bbox);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String text,  double confidence,  FrbBoundingBox bbox)  $default,) {final _that = this;
switch (_that) {
case _FrbOcrBlock():
return $default(_that.text,_that.confidence,_that.bbox);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String text,  double confidence,  FrbBoundingBox bbox)?  $default,) {final _that = this;
switch (_that) {
case _FrbOcrBlock() when $default != null:
return $default(_that.text,_that.confidence,_that.bbox);case _:
  return null;

}
}

}

/// @nodoc


class _FrbOcrBlock implements FrbOcrBlock {
  const _FrbOcrBlock({required this.text, required this.confidence, required this.bbox});
  

@override final  String text;
@override final  double confidence;
@override final  FrbBoundingBox bbox;

/// Create a copy of FrbOcrBlock
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$FrbOcrBlockCopyWith<_FrbOcrBlock> get copyWith => __$FrbOcrBlockCopyWithImpl<_FrbOcrBlock>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _FrbOcrBlock&&(identical(other.text, text) || other.text == text)&&(identical(other.confidence, confidence) || other.confidence == confidence)&&(identical(other.bbox, bbox) || other.bbox == bbox));
}


@override
int get hashCode => Object.hash(runtimeType,text,confidence,bbox);

@override
String toString() {
  return 'FrbOcrBlock(text: $text, confidence: $confidence, bbox: $bbox)';
}


}

/// @nodoc
abstract mixin class _$FrbOcrBlockCopyWith<$Res> implements $FrbOcrBlockCopyWith<$Res> {
  factory _$FrbOcrBlockCopyWith(_FrbOcrBlock value, $Res Function(_FrbOcrBlock) _then) = __$FrbOcrBlockCopyWithImpl;
@override @useResult
$Res call({
 String text, double confidence, FrbBoundingBox bbox
});


@override $FrbBoundingBoxCopyWith<$Res> get bbox;

}
/// @nodoc
class __$FrbOcrBlockCopyWithImpl<$Res>
    implements _$FrbOcrBlockCopyWith<$Res> {
  __$FrbOcrBlockCopyWithImpl(this._self, this._then);

  final _FrbOcrBlock _self;
  final $Res Function(_FrbOcrBlock) _then;

/// Create a copy of FrbOcrBlock
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? text = null,Object? confidence = null,Object? bbox = null,}) {
  return _then(_FrbOcrBlock(
text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,confidence: null == confidence ? _self.confidence : confidence // ignore: cast_nullable_to_non_nullable
as double,bbox: null == bbox ? _self.bbox : bbox // ignore: cast_nullable_to_non_nullable
as FrbBoundingBox,
  ));
}

/// Create a copy of FrbOcrBlock
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$FrbBoundingBoxCopyWith<$Res> get bbox {
  
  return $FrbBoundingBoxCopyWith<$Res>(_self.bbox, (value) {
    return _then(_self.copyWith(bbox: value));
  });
}
}

/// @nodoc
mixin _$FrbOcrResult {

 String get rawText; List<FrbOcrBlock> get blocks; double get confidence;
/// Create a copy of FrbOcrResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FrbOcrResultCopyWith<FrbOcrResult> get copyWith => _$FrbOcrResultCopyWithImpl<FrbOcrResult>(this as FrbOcrResult, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FrbOcrResult&&(identical(other.rawText, rawText) || other.rawText == rawText)&&const DeepCollectionEquality().equals(other.blocks, blocks)&&(identical(other.confidence, confidence) || other.confidence == confidence));
}


@override
int get hashCode => Object.hash(runtimeType,rawText,const DeepCollectionEquality().hash(blocks),confidence);

@override
String toString() {
  return 'FrbOcrResult(rawText: $rawText, blocks: $blocks, confidence: $confidence)';
}


}

/// @nodoc
abstract mixin class $FrbOcrResultCopyWith<$Res>  {
  factory $FrbOcrResultCopyWith(FrbOcrResult value, $Res Function(FrbOcrResult) _then) = _$FrbOcrResultCopyWithImpl;
@useResult
$Res call({
 String rawText, List<FrbOcrBlock> blocks, double confidence
});




}
/// @nodoc
class _$FrbOcrResultCopyWithImpl<$Res>
    implements $FrbOcrResultCopyWith<$Res> {
  _$FrbOcrResultCopyWithImpl(this._self, this._then);

  final FrbOcrResult _self;
  final $Res Function(FrbOcrResult) _then;

/// Create a copy of FrbOcrResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? rawText = null,Object? blocks = null,Object? confidence = null,}) {
  return _then(_self.copyWith(
rawText: null == rawText ? _self.rawText : rawText // ignore: cast_nullable_to_non_nullable
as String,blocks: null == blocks ? _self.blocks : blocks // ignore: cast_nullable_to_non_nullable
as List<FrbOcrBlock>,confidence: null == confidence ? _self.confidence : confidence // ignore: cast_nullable_to_non_nullable
as double,
  ));
}

}


/// Adds pattern-matching-related methods to [FrbOcrResult].
extension FrbOcrResultPatterns on FrbOcrResult {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _FrbOcrResult value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _FrbOcrResult() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _FrbOcrResult value)  $default,){
final _that = this;
switch (_that) {
case _FrbOcrResult():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _FrbOcrResult value)?  $default,){
final _that = this;
switch (_that) {
case _FrbOcrResult() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String rawText,  List<FrbOcrBlock> blocks,  double confidence)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _FrbOcrResult() when $default != null:
return $default(_that.rawText,_that.blocks,_that.confidence);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String rawText,  List<FrbOcrBlock> blocks,  double confidence)  $default,) {final _that = this;
switch (_that) {
case _FrbOcrResult():
return $default(_that.rawText,_that.blocks,_that.confidence);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String rawText,  List<FrbOcrBlock> blocks,  double confidence)?  $default,) {final _that = this;
switch (_that) {
case _FrbOcrResult() when $default != null:
return $default(_that.rawText,_that.blocks,_that.confidence);case _:
  return null;

}
}

}

/// @nodoc


class _FrbOcrResult implements FrbOcrResult {
  const _FrbOcrResult({required this.rawText, required final  List<FrbOcrBlock> blocks, required this.confidence}): _blocks = blocks;
  

@override final  String rawText;
 final  List<FrbOcrBlock> _blocks;
@override List<FrbOcrBlock> get blocks {
  if (_blocks is EqualUnmodifiableListView) return _blocks;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_blocks);
}

@override final  double confidence;

/// Create a copy of FrbOcrResult
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$FrbOcrResultCopyWith<_FrbOcrResult> get copyWith => __$FrbOcrResultCopyWithImpl<_FrbOcrResult>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _FrbOcrResult&&(identical(other.rawText, rawText) || other.rawText == rawText)&&const DeepCollectionEquality().equals(other._blocks, _blocks)&&(identical(other.confidence, confidence) || other.confidence == confidence));
}


@override
int get hashCode => Object.hash(runtimeType,rawText,const DeepCollectionEquality().hash(_blocks),confidence);

@override
String toString() {
  return 'FrbOcrResult(rawText: $rawText, blocks: $blocks, confidence: $confidence)';
}


}

/// @nodoc
abstract mixin class _$FrbOcrResultCopyWith<$Res> implements $FrbOcrResultCopyWith<$Res> {
  factory _$FrbOcrResultCopyWith(_FrbOcrResult value, $Res Function(_FrbOcrResult) _then) = __$FrbOcrResultCopyWithImpl;
@override @useResult
$Res call({
 String rawText, List<FrbOcrBlock> blocks, double confidence
});




}
/// @nodoc
class __$FrbOcrResultCopyWithImpl<$Res>
    implements _$FrbOcrResultCopyWith<$Res> {
  __$FrbOcrResultCopyWithImpl(this._self, this._then);

  final _FrbOcrResult _self;
  final $Res Function(_FrbOcrResult) _then;

/// Create a copy of FrbOcrResult
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? rawText = null,Object? blocks = null,Object? confidence = null,}) {
  return _then(_FrbOcrResult(
rawText: null == rawText ? _self.rawText : rawText // ignore: cast_nullable_to_non_nullable
as String,blocks: null == blocks ? _self._blocks : blocks // ignore: cast_nullable_to_non_nullable
as List<FrbOcrBlock>,confidence: null == confidence ? _self.confidence : confidence // ignore: cast_nullable_to_non_nullable
as double,
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
mixin _$OcrEngineStatus {

 bool get isLoaded; bool get detLoaded; bool get clsLoaded; bool get recLoaded; BigInt get uptimeSecs;
/// Create a copy of OcrEngineStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OcrEngineStatusCopyWith<OcrEngineStatus> get copyWith => _$OcrEngineStatusCopyWithImpl<OcrEngineStatus>(this as OcrEngineStatus, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OcrEngineStatus&&(identical(other.isLoaded, isLoaded) || other.isLoaded == isLoaded)&&(identical(other.detLoaded, detLoaded) || other.detLoaded == detLoaded)&&(identical(other.clsLoaded, clsLoaded) || other.clsLoaded == clsLoaded)&&(identical(other.recLoaded, recLoaded) || other.recLoaded == recLoaded)&&(identical(other.uptimeSecs, uptimeSecs) || other.uptimeSecs == uptimeSecs));
}


@override
int get hashCode => Object.hash(runtimeType,isLoaded,detLoaded,clsLoaded,recLoaded,uptimeSecs);

@override
String toString() {
  return 'OcrEngineStatus(isLoaded: $isLoaded, detLoaded: $detLoaded, clsLoaded: $clsLoaded, recLoaded: $recLoaded, uptimeSecs: $uptimeSecs)';
}


}

/// @nodoc
abstract mixin class $OcrEngineStatusCopyWith<$Res>  {
  factory $OcrEngineStatusCopyWith(OcrEngineStatus value, $Res Function(OcrEngineStatus) _then) = _$OcrEngineStatusCopyWithImpl;
@useResult
$Res call({
 bool isLoaded, bool detLoaded, bool clsLoaded, bool recLoaded, BigInt uptimeSecs
});




}
/// @nodoc
class _$OcrEngineStatusCopyWithImpl<$Res>
    implements $OcrEngineStatusCopyWith<$Res> {
  _$OcrEngineStatusCopyWithImpl(this._self, this._then);

  final OcrEngineStatus _self;
  final $Res Function(OcrEngineStatus) _then;

/// Create a copy of OcrEngineStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? isLoaded = null,Object? detLoaded = null,Object? clsLoaded = null,Object? recLoaded = null,Object? uptimeSecs = null,}) {
  return _then(_self.copyWith(
isLoaded: null == isLoaded ? _self.isLoaded : isLoaded // ignore: cast_nullable_to_non_nullable
as bool,detLoaded: null == detLoaded ? _self.detLoaded : detLoaded // ignore: cast_nullable_to_non_nullable
as bool,clsLoaded: null == clsLoaded ? _self.clsLoaded : clsLoaded // ignore: cast_nullable_to_non_nullable
as bool,recLoaded: null == recLoaded ? _self.recLoaded : recLoaded // ignore: cast_nullable_to_non_nullable
as bool,uptimeSecs: null == uptimeSecs ? _self.uptimeSecs : uptimeSecs // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}

}


/// Adds pattern-matching-related methods to [OcrEngineStatus].
extension OcrEngineStatusPatterns on OcrEngineStatus {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _OcrEngineStatus value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _OcrEngineStatus() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _OcrEngineStatus value)  $default,){
final _that = this;
switch (_that) {
case _OcrEngineStatus():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _OcrEngineStatus value)?  $default,){
final _that = this;
switch (_that) {
case _OcrEngineStatus() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( bool isLoaded,  bool detLoaded,  bool clsLoaded,  bool recLoaded,  BigInt uptimeSecs)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _OcrEngineStatus() when $default != null:
return $default(_that.isLoaded,_that.detLoaded,_that.clsLoaded,_that.recLoaded,_that.uptimeSecs);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( bool isLoaded,  bool detLoaded,  bool clsLoaded,  bool recLoaded,  BigInt uptimeSecs)  $default,) {final _that = this;
switch (_that) {
case _OcrEngineStatus():
return $default(_that.isLoaded,_that.detLoaded,_that.clsLoaded,_that.recLoaded,_that.uptimeSecs);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( bool isLoaded,  bool detLoaded,  bool clsLoaded,  bool recLoaded,  BigInt uptimeSecs)?  $default,) {final _that = this;
switch (_that) {
case _OcrEngineStatus() when $default != null:
return $default(_that.isLoaded,_that.detLoaded,_that.clsLoaded,_that.recLoaded,_that.uptimeSecs);case _:
  return null;

}
}

}

/// @nodoc


class _OcrEngineStatus implements OcrEngineStatus {
  const _OcrEngineStatus({required this.isLoaded, required this.detLoaded, required this.clsLoaded, required this.recLoaded, required this.uptimeSecs});
  

@override final  bool isLoaded;
@override final  bool detLoaded;
@override final  bool clsLoaded;
@override final  bool recLoaded;
@override final  BigInt uptimeSecs;

/// Create a copy of OcrEngineStatus
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$OcrEngineStatusCopyWith<_OcrEngineStatus> get copyWith => __$OcrEngineStatusCopyWithImpl<_OcrEngineStatus>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _OcrEngineStatus&&(identical(other.isLoaded, isLoaded) || other.isLoaded == isLoaded)&&(identical(other.detLoaded, detLoaded) || other.detLoaded == detLoaded)&&(identical(other.clsLoaded, clsLoaded) || other.clsLoaded == clsLoaded)&&(identical(other.recLoaded, recLoaded) || other.recLoaded == recLoaded)&&(identical(other.uptimeSecs, uptimeSecs) || other.uptimeSecs == uptimeSecs));
}


@override
int get hashCode => Object.hash(runtimeType,isLoaded,detLoaded,clsLoaded,recLoaded,uptimeSecs);

@override
String toString() {
  return 'OcrEngineStatus(isLoaded: $isLoaded, detLoaded: $detLoaded, clsLoaded: $clsLoaded, recLoaded: $recLoaded, uptimeSecs: $uptimeSecs)';
}


}

/// @nodoc
abstract mixin class _$OcrEngineStatusCopyWith<$Res> implements $OcrEngineStatusCopyWith<$Res> {
  factory _$OcrEngineStatusCopyWith(_OcrEngineStatus value, $Res Function(_OcrEngineStatus) _then) = __$OcrEngineStatusCopyWithImpl;
@override @useResult
$Res call({
 bool isLoaded, bool detLoaded, bool clsLoaded, bool recLoaded, BigInt uptimeSecs
});




}
/// @nodoc
class __$OcrEngineStatusCopyWithImpl<$Res>
    implements _$OcrEngineStatusCopyWith<$Res> {
  __$OcrEngineStatusCopyWithImpl(this._self, this._then);

  final _OcrEngineStatus _self;
  final $Res Function(_OcrEngineStatus) _then;

/// Create a copy of OcrEngineStatus
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? isLoaded = null,Object? detLoaded = null,Object? clsLoaded = null,Object? recLoaded = null,Object? uptimeSecs = null,}) {
  return _then(_OcrEngineStatus(
isLoaded: null == isLoaded ? _self.isLoaded : isLoaded // ignore: cast_nullable_to_non_nullable
as bool,detLoaded: null == detLoaded ? _self.detLoaded : detLoaded // ignore: cast_nullable_to_non_nullable
as bool,clsLoaded: null == clsLoaded ? _self.clsLoaded : clsLoaded // ignore: cast_nullable_to_non_nullable
as bool,recLoaded: null == recLoaded ? _self.recLoaded : recLoaded // ignore: cast_nullable_to_non_nullable
as bool,uptimeSecs: null == uptimeSecs ? _self.uptimeSecs : uptimeSecs // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc
mixin _$PluginSessionInfo {

 String get sessionId; String get pluginId; String get pluginName; PlatformInt64 get startedAtSecs; PlatformInt64 get expiresAtSecs;
/// Create a copy of PluginSessionInfo
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PluginSessionInfoCopyWith<PluginSessionInfo> get copyWith => _$PluginSessionInfoCopyWithImpl<PluginSessionInfo>(this as PluginSessionInfo, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PluginSessionInfo&&(identical(other.sessionId, sessionId) || other.sessionId == sessionId)&&(identical(other.pluginId, pluginId) || other.pluginId == pluginId)&&(identical(other.pluginName, pluginName) || other.pluginName == pluginName)&&(identical(other.startedAtSecs, startedAtSecs) || other.startedAtSecs == startedAtSecs)&&(identical(other.expiresAtSecs, expiresAtSecs) || other.expiresAtSecs == expiresAtSecs));
}


@override
int get hashCode => Object.hash(runtimeType,sessionId,pluginId,pluginName,startedAtSecs,expiresAtSecs);

@override
String toString() {
  return 'PluginSessionInfo(sessionId: $sessionId, pluginId: $pluginId, pluginName: $pluginName, startedAtSecs: $startedAtSecs, expiresAtSecs: $expiresAtSecs)';
}


}

/// @nodoc
abstract mixin class $PluginSessionInfoCopyWith<$Res>  {
  factory $PluginSessionInfoCopyWith(PluginSessionInfo value, $Res Function(PluginSessionInfo) _then) = _$PluginSessionInfoCopyWithImpl;
@useResult
$Res call({
 String sessionId, String pluginId, String pluginName, PlatformInt64 startedAtSecs, PlatformInt64 expiresAtSecs
});




}
/// @nodoc
class _$PluginSessionInfoCopyWithImpl<$Res>
    implements $PluginSessionInfoCopyWith<$Res> {
  _$PluginSessionInfoCopyWithImpl(this._self, this._then);

  final PluginSessionInfo _self;
  final $Res Function(PluginSessionInfo) _then;

/// Create a copy of PluginSessionInfo
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? sessionId = null,Object? pluginId = null,Object? pluginName = null,Object? startedAtSecs = null,Object? expiresAtSecs = null,}) {
  return _then(_self.copyWith(
sessionId: null == sessionId ? _self.sessionId : sessionId // ignore: cast_nullable_to_non_nullable
as String,pluginId: null == pluginId ? _self.pluginId : pluginId // ignore: cast_nullable_to_non_nullable
as String,pluginName: null == pluginName ? _self.pluginName : pluginName // ignore: cast_nullable_to_non_nullable
as String,startedAtSecs: null == startedAtSecs ? _self.startedAtSecs : startedAtSecs // ignore: cast_nullable_to_non_nullable
as PlatformInt64,expiresAtSecs: null == expiresAtSecs ? _self.expiresAtSecs : expiresAtSecs // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}

}


/// Adds pattern-matching-related methods to [PluginSessionInfo].
extension PluginSessionInfoPatterns on PluginSessionInfo {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _PluginSessionInfo value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _PluginSessionInfo() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _PluginSessionInfo value)  $default,){
final _that = this;
switch (_that) {
case _PluginSessionInfo():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _PluginSessionInfo value)?  $default,){
final _that = this;
switch (_that) {
case _PluginSessionInfo() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String sessionId,  String pluginId,  String pluginName,  PlatformInt64 startedAtSecs,  PlatformInt64 expiresAtSecs)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _PluginSessionInfo() when $default != null:
return $default(_that.sessionId,_that.pluginId,_that.pluginName,_that.startedAtSecs,_that.expiresAtSecs);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String sessionId,  String pluginId,  String pluginName,  PlatformInt64 startedAtSecs,  PlatformInt64 expiresAtSecs)  $default,) {final _that = this;
switch (_that) {
case _PluginSessionInfo():
return $default(_that.sessionId,_that.pluginId,_that.pluginName,_that.startedAtSecs,_that.expiresAtSecs);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String sessionId,  String pluginId,  String pluginName,  PlatformInt64 startedAtSecs,  PlatformInt64 expiresAtSecs)?  $default,) {final _that = this;
switch (_that) {
case _PluginSessionInfo() when $default != null:
return $default(_that.sessionId,_that.pluginId,_that.pluginName,_that.startedAtSecs,_that.expiresAtSecs);case _:
  return null;

}
}

}

/// @nodoc


class _PluginSessionInfo implements PluginSessionInfo {
  const _PluginSessionInfo({required this.sessionId, required this.pluginId, required this.pluginName, required this.startedAtSecs, required this.expiresAtSecs});
  

@override final  String sessionId;
@override final  String pluginId;
@override final  String pluginName;
@override final  PlatformInt64 startedAtSecs;
@override final  PlatformInt64 expiresAtSecs;

/// Create a copy of PluginSessionInfo
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$PluginSessionInfoCopyWith<_PluginSessionInfo> get copyWith => __$PluginSessionInfoCopyWithImpl<_PluginSessionInfo>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _PluginSessionInfo&&(identical(other.sessionId, sessionId) || other.sessionId == sessionId)&&(identical(other.pluginId, pluginId) || other.pluginId == pluginId)&&(identical(other.pluginName, pluginName) || other.pluginName == pluginName)&&(identical(other.startedAtSecs, startedAtSecs) || other.startedAtSecs == startedAtSecs)&&(identical(other.expiresAtSecs, expiresAtSecs) || other.expiresAtSecs == expiresAtSecs));
}


@override
int get hashCode => Object.hash(runtimeType,sessionId,pluginId,pluginName,startedAtSecs,expiresAtSecs);

@override
String toString() {
  return 'PluginSessionInfo(sessionId: $sessionId, pluginId: $pluginId, pluginName: $pluginName, startedAtSecs: $startedAtSecs, expiresAtSecs: $expiresAtSecs)';
}


}

/// @nodoc
abstract mixin class _$PluginSessionInfoCopyWith<$Res> implements $PluginSessionInfoCopyWith<$Res> {
  factory _$PluginSessionInfoCopyWith(_PluginSessionInfo value, $Res Function(_PluginSessionInfo) _then) = __$PluginSessionInfoCopyWithImpl;
@override @useResult
$Res call({
 String sessionId, String pluginId, String pluginName, PlatformInt64 startedAtSecs, PlatformInt64 expiresAtSecs
});




}
/// @nodoc
class __$PluginSessionInfoCopyWithImpl<$Res>
    implements _$PluginSessionInfoCopyWith<$Res> {
  __$PluginSessionInfoCopyWithImpl(this._self, this._then);

  final _PluginSessionInfo _self;
  final $Res Function(_PluginSessionInfo) _then;

/// Create a copy of PluginSessionInfo
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? sessionId = null,Object? pluginId = null,Object? pluginName = null,Object? startedAtSecs = null,Object? expiresAtSecs = null,}) {
  return _then(_PluginSessionInfo(
sessionId: null == sessionId ? _self.sessionId : sessionId // ignore: cast_nullable_to_non_nullable
as String,pluginId: null == pluginId ? _self.pluginId : pluginId // ignore: cast_nullable_to_non_nullable
as String,pluginName: null == pluginName ? _self.pluginName : pluginName // ignore: cast_nullable_to_non_nullable
as String,startedAtSecs: null == startedAtSecs ? _self.startedAtSecs : startedAtSecs // ignore: cast_nullable_to_non_nullable
as PlatformInt64,expiresAtSecs: null == expiresAtSecs ? _self.expiresAtSecs : expiresAtSecs // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
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
mixin _$SyncResult {

 bool get success; SyncDirection get direction; BigInt get bytesSent; BigInt get bytesReceived; String? get error;
/// Create a copy of SyncResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncResultCopyWith<SyncResult> get copyWith => _$SyncResultCopyWithImpl<SyncResult>(this as SyncResult, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncResult&&(identical(other.success, success) || other.success == success)&&(identical(other.direction, direction) || other.direction == direction)&&(identical(other.bytesSent, bytesSent) || other.bytesSent == bytesSent)&&(identical(other.bytesReceived, bytesReceived) || other.bytesReceived == bytesReceived)&&(identical(other.error, error) || other.error == error));
}


@override
int get hashCode => Object.hash(runtimeType,success,direction,bytesSent,bytesReceived,error);

@override
String toString() {
  return 'SyncResult(success: $success, direction: $direction, bytesSent: $bytesSent, bytesReceived: $bytesReceived, error: $error)';
}


}

/// @nodoc
abstract mixin class $SyncResultCopyWith<$Res>  {
  factory $SyncResultCopyWith(SyncResult value, $Res Function(SyncResult) _then) = _$SyncResultCopyWithImpl;
@useResult
$Res call({
 bool success, SyncDirection direction, BigInt bytesSent, BigInt bytesReceived, String? error
});




}
/// @nodoc
class _$SyncResultCopyWithImpl<$Res>
    implements $SyncResultCopyWith<$Res> {
  _$SyncResultCopyWithImpl(this._self, this._then);

  final SyncResult _self;
  final $Res Function(SyncResult) _then;

/// Create a copy of SyncResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? success = null,Object? direction = null,Object? bytesSent = null,Object? bytesReceived = null,Object? error = freezed,}) {
  return _then(_self.copyWith(
success: null == success ? _self.success : success // ignore: cast_nullable_to_non_nullable
as bool,direction: null == direction ? _self.direction : direction // ignore: cast_nullable_to_non_nullable
as SyncDirection,bytesSent: null == bytesSent ? _self.bytesSent : bytesSent // ignore: cast_nullable_to_non_nullable
as BigInt,bytesReceived: null == bytesReceived ? _self.bytesReceived : bytesReceived // ignore: cast_nullable_to_non_nullable
as BigInt,error: freezed == error ? _self.error : error // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}

}


/// Adds pattern-matching-related methods to [SyncResult].
extension SyncResultPatterns on SyncResult {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _SyncResult value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _SyncResult() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _SyncResult value)  $default,){
final _that = this;
switch (_that) {
case _SyncResult():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _SyncResult value)?  $default,){
final _that = this;
switch (_that) {
case _SyncResult() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( bool success,  SyncDirection direction,  BigInt bytesSent,  BigInt bytesReceived,  String? error)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _SyncResult() when $default != null:
return $default(_that.success,_that.direction,_that.bytesSent,_that.bytesReceived,_that.error);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( bool success,  SyncDirection direction,  BigInt bytesSent,  BigInt bytesReceived,  String? error)  $default,) {final _that = this;
switch (_that) {
case _SyncResult():
return $default(_that.success,_that.direction,_that.bytesSent,_that.bytesReceived,_that.error);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( bool success,  SyncDirection direction,  BigInt bytesSent,  BigInt bytesReceived,  String? error)?  $default,) {final _that = this;
switch (_that) {
case _SyncResult() when $default != null:
return $default(_that.success,_that.direction,_that.bytesSent,_that.bytesReceived,_that.error);case _:
  return null;

}
}

}

/// @nodoc


class _SyncResult implements SyncResult {
  const _SyncResult({required this.success, required this.direction, required this.bytesSent, required this.bytesReceived, this.error});
  

@override final  bool success;
@override final  SyncDirection direction;
@override final  BigInt bytesSent;
@override final  BigInt bytesReceived;
@override final  String? error;

/// Create a copy of SyncResult
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$SyncResultCopyWith<_SyncResult> get copyWith => __$SyncResultCopyWithImpl<_SyncResult>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _SyncResult&&(identical(other.success, success) || other.success == success)&&(identical(other.direction, direction) || other.direction == direction)&&(identical(other.bytesSent, bytesSent) || other.bytesSent == bytesSent)&&(identical(other.bytesReceived, bytesReceived) || other.bytesReceived == bytesReceived)&&(identical(other.error, error) || other.error == error));
}


@override
int get hashCode => Object.hash(runtimeType,success,direction,bytesSent,bytesReceived,error);

@override
String toString() {
  return 'SyncResult(success: $success, direction: $direction, bytesSent: $bytesSent, bytesReceived: $bytesReceived, error: $error)';
}


}

/// @nodoc
abstract mixin class _$SyncResultCopyWith<$Res> implements $SyncResultCopyWith<$Res> {
  factory _$SyncResultCopyWith(_SyncResult value, $Res Function(_SyncResult) _then) = __$SyncResultCopyWithImpl;
@override @useResult
$Res call({
 bool success, SyncDirection direction, BigInt bytesSent, BigInt bytesReceived, String? error
});




}
/// @nodoc
class __$SyncResultCopyWithImpl<$Res>
    implements _$SyncResultCopyWith<$Res> {
  __$SyncResultCopyWithImpl(this._self, this._then);

  final _SyncResult _self;
  final $Res Function(_SyncResult) _then;

/// Create a copy of SyncResult
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? success = null,Object? direction = null,Object? bytesSent = null,Object? bytesReceived = null,Object? error = freezed,}) {
  return _then(_SyncResult(
success: null == success ? _self.success : success // ignore: cast_nullable_to_non_nullable
as bool,direction: null == direction ? _self.direction : direction // ignore: cast_nullable_to_non_nullable
as SyncDirection,bytesSent: null == bytesSent ? _self.bytesSent : bytesSent // ignore: cast_nullable_to_non_nullable
as BigInt,bytesReceived: null == bytesReceived ? _self.bytesReceived : bytesReceived // ignore: cast_nullable_to_non_nullable
as BigInt,error: freezed == error ? _self.error : error // ignore: cast_nullable_to_non_nullable
as String?,
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
