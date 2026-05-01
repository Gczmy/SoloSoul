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

// dart format on
