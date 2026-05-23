// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'manager.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$PluginEvent {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PluginEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'PluginEvent()';
}


}

/// @nodoc
class $PluginEventCopyWith<$Res>  {
$PluginEventCopyWith(PluginEvent _, $Res Function(PluginEvent) __);
}


/// Adds pattern-matching-related methods to [PluginEvent].
extension PluginEventPatterns on PluginEvent {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( PluginEvent_ConsentRequest value)?  consentRequest,TResult Function( PluginEvent_ConsentTimeout value)?  consentTimeout,TResult Function( PluginEvent_Log value)?  log,TResult Function( PluginEvent_Progress value)?  progress,TResult Function( PluginEvent_Completed value)?  completed,TResult Function( PluginEvent_Error value)?  error,required TResult orElse(),}){
final _that = this;
switch (_that) {
case PluginEvent_ConsentRequest() when consentRequest != null:
return consentRequest(_that);case PluginEvent_ConsentTimeout() when consentTimeout != null:
return consentTimeout(_that);case PluginEvent_Log() when log != null:
return log(_that);case PluginEvent_Progress() when progress != null:
return progress(_that);case PluginEvent_Completed() when completed != null:
return completed(_that);case PluginEvent_Error() when error != null:
return error(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( PluginEvent_ConsentRequest value)  consentRequest,required TResult Function( PluginEvent_ConsentTimeout value)  consentTimeout,required TResult Function( PluginEvent_Log value)  log,required TResult Function( PluginEvent_Progress value)  progress,required TResult Function( PluginEvent_Completed value)  completed,required TResult Function( PluginEvent_Error value)  error,}){
final _that = this;
switch (_that) {
case PluginEvent_ConsentRequest():
return consentRequest(_that);case PluginEvent_ConsentTimeout():
return consentTimeout(_that);case PluginEvent_Log():
return log(_that);case PluginEvent_Progress():
return progress(_that);case PluginEvent_Completed():
return completed(_that);case PluginEvent_Error():
return error(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( PluginEvent_ConsentRequest value)?  consentRequest,TResult? Function( PluginEvent_ConsentTimeout value)?  consentTimeout,TResult? Function( PluginEvent_Log value)?  log,TResult? Function( PluginEvent_Progress value)?  progress,TResult? Function( PluginEvent_Completed value)?  completed,TResult? Function( PluginEvent_Error value)?  error,}){
final _that = this;
switch (_that) {
case PluginEvent_ConsentRequest() when consentRequest != null:
return consentRequest(_that);case PluginEvent_ConsentTimeout() when consentTimeout != null:
return consentTimeout(_that);case PluginEvent_Log() when log != null:
return log(_that);case PluginEvent_Progress() when progress != null:
return progress(_that);case PluginEvent_Completed() when completed != null:
return completed(_that);case PluginEvent_Error() when error != null:
return error(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String requestId,  String pluginId,  String pluginName,  String field,  String sensitivity)?  consentRequest,TResult Function( String requestId)?  consentTimeout,TResult Function( String level,  String message)?  log,TResult Function( int percent)?  progress,TResult Function( int exitCode)?  completed,TResult Function( String message)?  error,required TResult orElse(),}) {final _that = this;
switch (_that) {
case PluginEvent_ConsentRequest() when consentRequest != null:
return consentRequest(_that.requestId,_that.pluginId,_that.pluginName,_that.field,_that.sensitivity);case PluginEvent_ConsentTimeout() when consentTimeout != null:
return consentTimeout(_that.requestId);case PluginEvent_Log() when log != null:
return log(_that.level,_that.message);case PluginEvent_Progress() when progress != null:
return progress(_that.percent);case PluginEvent_Completed() when completed != null:
return completed(_that.exitCode);case PluginEvent_Error() when error != null:
return error(_that.message);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String requestId,  String pluginId,  String pluginName,  String field,  String sensitivity)  consentRequest,required TResult Function( String requestId)  consentTimeout,required TResult Function( String level,  String message)  log,required TResult Function( int percent)  progress,required TResult Function( int exitCode)  completed,required TResult Function( String message)  error,}) {final _that = this;
switch (_that) {
case PluginEvent_ConsentRequest():
return consentRequest(_that.requestId,_that.pluginId,_that.pluginName,_that.field,_that.sensitivity);case PluginEvent_ConsentTimeout():
return consentTimeout(_that.requestId);case PluginEvent_Log():
return log(_that.level,_that.message);case PluginEvent_Progress():
return progress(_that.percent);case PluginEvent_Completed():
return completed(_that.exitCode);case PluginEvent_Error():
return error(_that.message);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String requestId,  String pluginId,  String pluginName,  String field,  String sensitivity)?  consentRequest,TResult? Function( String requestId)?  consentTimeout,TResult? Function( String level,  String message)?  log,TResult? Function( int percent)?  progress,TResult? Function( int exitCode)?  completed,TResult? Function( String message)?  error,}) {final _that = this;
switch (_that) {
case PluginEvent_ConsentRequest() when consentRequest != null:
return consentRequest(_that.requestId,_that.pluginId,_that.pluginName,_that.field,_that.sensitivity);case PluginEvent_ConsentTimeout() when consentTimeout != null:
return consentTimeout(_that.requestId);case PluginEvent_Log() when log != null:
return log(_that.level,_that.message);case PluginEvent_Progress() when progress != null:
return progress(_that.percent);case PluginEvent_Completed() when completed != null:
return completed(_that.exitCode);case PluginEvent_Error() when error != null:
return error(_that.message);case _:
  return null;

}
}

}

/// @nodoc


class PluginEvent_ConsentRequest extends PluginEvent {
  const PluginEvent_ConsentRequest({required this.requestId, required this.pluginId, required this.pluginName, required this.field, required this.sensitivity}): super._();
  

 final  String requestId;
 final  String pluginId;
 final  String pluginName;
 final  String field;
 final  String sensitivity;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PluginEvent_ConsentRequestCopyWith<PluginEvent_ConsentRequest> get copyWith => _$PluginEvent_ConsentRequestCopyWithImpl<PluginEvent_ConsentRequest>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PluginEvent_ConsentRequest&&(identical(other.requestId, requestId) || other.requestId == requestId)&&(identical(other.pluginId, pluginId) || other.pluginId == pluginId)&&(identical(other.pluginName, pluginName) || other.pluginName == pluginName)&&(identical(other.field, field) || other.field == field)&&(identical(other.sensitivity, sensitivity) || other.sensitivity == sensitivity));
}


@override
int get hashCode => Object.hash(runtimeType,requestId,pluginId,pluginName,field,sensitivity);

@override
String toString() {
  return 'PluginEvent.consentRequest(requestId: $requestId, pluginId: $pluginId, pluginName: $pluginName, field: $field, sensitivity: $sensitivity)';
}


}

/// @nodoc
abstract mixin class $PluginEvent_ConsentRequestCopyWith<$Res> implements $PluginEventCopyWith<$Res> {
  factory $PluginEvent_ConsentRequestCopyWith(PluginEvent_ConsentRequest value, $Res Function(PluginEvent_ConsentRequest) _then) = _$PluginEvent_ConsentRequestCopyWithImpl;
@useResult
$Res call({
 String requestId, String pluginId, String pluginName, String field, String sensitivity
});




}
/// @nodoc
class _$PluginEvent_ConsentRequestCopyWithImpl<$Res>
    implements $PluginEvent_ConsentRequestCopyWith<$Res> {
  _$PluginEvent_ConsentRequestCopyWithImpl(this._self, this._then);

  final PluginEvent_ConsentRequest _self;
  final $Res Function(PluginEvent_ConsentRequest) _then;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? requestId = null,Object? pluginId = null,Object? pluginName = null,Object? field = null,Object? sensitivity = null,}) {
  return _then(PluginEvent_ConsentRequest(
requestId: null == requestId ? _self.requestId : requestId // ignore: cast_nullable_to_non_nullable
as String,pluginId: null == pluginId ? _self.pluginId : pluginId // ignore: cast_nullable_to_non_nullable
as String,pluginName: null == pluginName ? _self.pluginName : pluginName // ignore: cast_nullable_to_non_nullable
as String,field: null == field ? _self.field : field // ignore: cast_nullable_to_non_nullable
as String,sensitivity: null == sensitivity ? _self.sensitivity : sensitivity // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PluginEvent_ConsentTimeout extends PluginEvent {
  const PluginEvent_ConsentTimeout({required this.requestId}): super._();
  

 final  String requestId;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PluginEvent_ConsentTimeoutCopyWith<PluginEvent_ConsentTimeout> get copyWith => _$PluginEvent_ConsentTimeoutCopyWithImpl<PluginEvent_ConsentTimeout>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PluginEvent_ConsentTimeout&&(identical(other.requestId, requestId) || other.requestId == requestId));
}


@override
int get hashCode => Object.hash(runtimeType,requestId);

@override
String toString() {
  return 'PluginEvent.consentTimeout(requestId: $requestId)';
}


}

/// @nodoc
abstract mixin class $PluginEvent_ConsentTimeoutCopyWith<$Res> implements $PluginEventCopyWith<$Res> {
  factory $PluginEvent_ConsentTimeoutCopyWith(PluginEvent_ConsentTimeout value, $Res Function(PluginEvent_ConsentTimeout) _then) = _$PluginEvent_ConsentTimeoutCopyWithImpl;
@useResult
$Res call({
 String requestId
});




}
/// @nodoc
class _$PluginEvent_ConsentTimeoutCopyWithImpl<$Res>
    implements $PluginEvent_ConsentTimeoutCopyWith<$Res> {
  _$PluginEvent_ConsentTimeoutCopyWithImpl(this._self, this._then);

  final PluginEvent_ConsentTimeout _self;
  final $Res Function(PluginEvent_ConsentTimeout) _then;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? requestId = null,}) {
  return _then(PluginEvent_ConsentTimeout(
requestId: null == requestId ? _self.requestId : requestId // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PluginEvent_Log extends PluginEvent {
  const PluginEvent_Log({required this.level, required this.message}): super._();
  

 final  String level;
 final  String message;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PluginEvent_LogCopyWith<PluginEvent_Log> get copyWith => _$PluginEvent_LogCopyWithImpl<PluginEvent_Log>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PluginEvent_Log&&(identical(other.level, level) || other.level == level)&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,level,message);

@override
String toString() {
  return 'PluginEvent.log(level: $level, message: $message)';
}


}

/// @nodoc
abstract mixin class $PluginEvent_LogCopyWith<$Res> implements $PluginEventCopyWith<$Res> {
  factory $PluginEvent_LogCopyWith(PluginEvent_Log value, $Res Function(PluginEvent_Log) _then) = _$PluginEvent_LogCopyWithImpl;
@useResult
$Res call({
 String level, String message
});




}
/// @nodoc
class _$PluginEvent_LogCopyWithImpl<$Res>
    implements $PluginEvent_LogCopyWith<$Res> {
  _$PluginEvent_LogCopyWithImpl(this._self, this._then);

  final PluginEvent_Log _self;
  final $Res Function(PluginEvent_Log) _then;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? level = null,Object? message = null,}) {
  return _then(PluginEvent_Log(
level: null == level ? _self.level : level // ignore: cast_nullable_to_non_nullable
as String,message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PluginEvent_Progress extends PluginEvent {
  const PluginEvent_Progress({required this.percent}): super._();
  

 final  int percent;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PluginEvent_ProgressCopyWith<PluginEvent_Progress> get copyWith => _$PluginEvent_ProgressCopyWithImpl<PluginEvent_Progress>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PluginEvent_Progress&&(identical(other.percent, percent) || other.percent == percent));
}


@override
int get hashCode => Object.hash(runtimeType,percent);

@override
String toString() {
  return 'PluginEvent.progress(percent: $percent)';
}


}

/// @nodoc
abstract mixin class $PluginEvent_ProgressCopyWith<$Res> implements $PluginEventCopyWith<$Res> {
  factory $PluginEvent_ProgressCopyWith(PluginEvent_Progress value, $Res Function(PluginEvent_Progress) _then) = _$PluginEvent_ProgressCopyWithImpl;
@useResult
$Res call({
 int percent
});




}
/// @nodoc
class _$PluginEvent_ProgressCopyWithImpl<$Res>
    implements $PluginEvent_ProgressCopyWith<$Res> {
  _$PluginEvent_ProgressCopyWithImpl(this._self, this._then);

  final PluginEvent_Progress _self;
  final $Res Function(PluginEvent_Progress) _then;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? percent = null,}) {
  return _then(PluginEvent_Progress(
percent: null == percent ? _self.percent : percent // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class PluginEvent_Completed extends PluginEvent {
  const PluginEvent_Completed({required this.exitCode}): super._();
  

 final  int exitCode;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PluginEvent_CompletedCopyWith<PluginEvent_Completed> get copyWith => _$PluginEvent_CompletedCopyWithImpl<PluginEvent_Completed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PluginEvent_Completed&&(identical(other.exitCode, exitCode) || other.exitCode == exitCode));
}


@override
int get hashCode => Object.hash(runtimeType,exitCode);

@override
String toString() {
  return 'PluginEvent.completed(exitCode: $exitCode)';
}


}

/// @nodoc
abstract mixin class $PluginEvent_CompletedCopyWith<$Res> implements $PluginEventCopyWith<$Res> {
  factory $PluginEvent_CompletedCopyWith(PluginEvent_Completed value, $Res Function(PluginEvent_Completed) _then) = _$PluginEvent_CompletedCopyWithImpl;
@useResult
$Res call({
 int exitCode
});




}
/// @nodoc
class _$PluginEvent_CompletedCopyWithImpl<$Res>
    implements $PluginEvent_CompletedCopyWith<$Res> {
  _$PluginEvent_CompletedCopyWithImpl(this._self, this._then);

  final PluginEvent_Completed _self;
  final $Res Function(PluginEvent_Completed) _then;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? exitCode = null,}) {
  return _then(PluginEvent_Completed(
exitCode: null == exitCode ? _self.exitCode : exitCode // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class PluginEvent_Error extends PluginEvent {
  const PluginEvent_Error({required this.message}): super._();
  

 final  String message;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PluginEvent_ErrorCopyWith<PluginEvent_Error> get copyWith => _$PluginEvent_ErrorCopyWithImpl<PluginEvent_Error>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PluginEvent_Error&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'PluginEvent.error(message: $message)';
}


}

/// @nodoc
abstract mixin class $PluginEvent_ErrorCopyWith<$Res> implements $PluginEventCopyWith<$Res> {
  factory $PluginEvent_ErrorCopyWith(PluginEvent_Error value, $Res Function(PluginEvent_Error) _then) = _$PluginEvent_ErrorCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$PluginEvent_ErrorCopyWithImpl<$Res>
    implements $PluginEvent_ErrorCopyWith<$Res> {
  _$PluginEvent_ErrorCopyWithImpl(this._self, this._then);

  final PluginEvent_Error _self;
  final $Res Function(PluginEvent_Error) _then;

/// Create a copy of PluginEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(PluginEvent_Error(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
