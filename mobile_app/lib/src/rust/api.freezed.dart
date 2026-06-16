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
mixin _$MobileWatermarkError {

 String get field0;
/// Create a copy of MobileWatermarkError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MobileWatermarkErrorCopyWith<MobileWatermarkError> get copyWith => _$MobileWatermarkErrorCopyWithImpl<MobileWatermarkError>(this as MobileWatermarkError, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MobileWatermarkError&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'MobileWatermarkError(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $MobileWatermarkErrorCopyWith<$Res>  {
  factory $MobileWatermarkErrorCopyWith(MobileWatermarkError value, $Res Function(MobileWatermarkError) _then) = _$MobileWatermarkErrorCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$MobileWatermarkErrorCopyWithImpl<$Res>
    implements $MobileWatermarkErrorCopyWith<$Res> {
  _$MobileWatermarkErrorCopyWithImpl(this._self, this._then);

  final MobileWatermarkError _self;
  final $Res Function(MobileWatermarkError) _then;

/// Create a copy of MobileWatermarkError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? field0 = null,}) {
  return _then(_self.copyWith(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [MobileWatermarkError].
extension MobileWatermarkErrorPatterns on MobileWatermarkError {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( MobileWatermarkError_InvalidPayload value)?  invalidPayload,TResult Function( MobileWatermarkError_OperationFailed value)?  operationFailed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case MobileWatermarkError_InvalidPayload() when invalidPayload != null:
return invalidPayload(_that);case MobileWatermarkError_OperationFailed() when operationFailed != null:
return operationFailed(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( MobileWatermarkError_InvalidPayload value)  invalidPayload,required TResult Function( MobileWatermarkError_OperationFailed value)  operationFailed,}){
final _that = this;
switch (_that) {
case MobileWatermarkError_InvalidPayload():
return invalidPayload(_that);case MobileWatermarkError_OperationFailed():
return operationFailed(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( MobileWatermarkError_InvalidPayload value)?  invalidPayload,TResult? Function( MobileWatermarkError_OperationFailed value)?  operationFailed,}){
final _that = this;
switch (_that) {
case MobileWatermarkError_InvalidPayload() when invalidPayload != null:
return invalidPayload(_that);case MobileWatermarkError_OperationFailed() when operationFailed != null:
return operationFailed(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  invalidPayload,TResult Function( String field0)?  operationFailed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case MobileWatermarkError_InvalidPayload() when invalidPayload != null:
return invalidPayload(_that.field0);case MobileWatermarkError_OperationFailed() when operationFailed != null:
return operationFailed(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  invalidPayload,required TResult Function( String field0)  operationFailed,}) {final _that = this;
switch (_that) {
case MobileWatermarkError_InvalidPayload():
return invalidPayload(_that.field0);case MobileWatermarkError_OperationFailed():
return operationFailed(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  invalidPayload,TResult? Function( String field0)?  operationFailed,}) {final _that = this;
switch (_that) {
case MobileWatermarkError_InvalidPayload() when invalidPayload != null:
return invalidPayload(_that.field0);case MobileWatermarkError_OperationFailed() when operationFailed != null:
return operationFailed(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class MobileWatermarkError_InvalidPayload extends MobileWatermarkError {
  const MobileWatermarkError_InvalidPayload(this.field0): super._();
  

@override final  String field0;

/// Create a copy of MobileWatermarkError
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MobileWatermarkError_InvalidPayloadCopyWith<MobileWatermarkError_InvalidPayload> get copyWith => _$MobileWatermarkError_InvalidPayloadCopyWithImpl<MobileWatermarkError_InvalidPayload>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MobileWatermarkError_InvalidPayload&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'MobileWatermarkError.invalidPayload(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $MobileWatermarkError_InvalidPayloadCopyWith<$Res> implements $MobileWatermarkErrorCopyWith<$Res> {
  factory $MobileWatermarkError_InvalidPayloadCopyWith(MobileWatermarkError_InvalidPayload value, $Res Function(MobileWatermarkError_InvalidPayload) _then) = _$MobileWatermarkError_InvalidPayloadCopyWithImpl;
@override @useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$MobileWatermarkError_InvalidPayloadCopyWithImpl<$Res>
    implements $MobileWatermarkError_InvalidPayloadCopyWith<$Res> {
  _$MobileWatermarkError_InvalidPayloadCopyWithImpl(this._self, this._then);

  final MobileWatermarkError_InvalidPayload _self;
  final $Res Function(MobileWatermarkError_InvalidPayload) _then;

/// Create a copy of MobileWatermarkError
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(MobileWatermarkError_InvalidPayload(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class MobileWatermarkError_OperationFailed extends MobileWatermarkError {
  const MobileWatermarkError_OperationFailed(this.field0): super._();
  

@override final  String field0;

/// Create a copy of MobileWatermarkError
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MobileWatermarkError_OperationFailedCopyWith<MobileWatermarkError_OperationFailed> get copyWith => _$MobileWatermarkError_OperationFailedCopyWithImpl<MobileWatermarkError_OperationFailed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MobileWatermarkError_OperationFailed&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'MobileWatermarkError.operationFailed(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $MobileWatermarkError_OperationFailedCopyWith<$Res> implements $MobileWatermarkErrorCopyWith<$Res> {
  factory $MobileWatermarkError_OperationFailedCopyWith(MobileWatermarkError_OperationFailed value, $Res Function(MobileWatermarkError_OperationFailed) _then) = _$MobileWatermarkError_OperationFailedCopyWithImpl;
@override @useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$MobileWatermarkError_OperationFailedCopyWithImpl<$Res>
    implements $MobileWatermarkError_OperationFailedCopyWith<$Res> {
  _$MobileWatermarkError_OperationFailedCopyWithImpl(this._self, this._then);

  final MobileWatermarkError_OperationFailed _self;
  final $Res Function(MobileWatermarkError_OperationFailed) _then;

/// Create a copy of MobileWatermarkError
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(MobileWatermarkError_OperationFailed(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
