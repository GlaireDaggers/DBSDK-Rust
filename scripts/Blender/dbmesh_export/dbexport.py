# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful, but
# WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTIBILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
# General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <http://www.gnu.org/licenses/>.

bl_info = {
    "name" : "DreamBox Mesh Export",
    "author" : "Hazel Stagner",
    "description" : "Addon to export mesh files for use with the DreamBox SDK",
    "blender" : (2, 80, 0),
    "version" : (0, 0, 1),
    "location" : "",
    "warning" : "",
    "category" : "Export"
}

import re
import bpy
import bmesh
import struct
import mathutils

from math import degrees, radians

def blender_to_db_mat():
    return mathutils.Matrix.Rotation(radians(-90.0), 4, 'X') @ mathutils.Matrix.Rotation(radians(180.0), 4, 'Z')

def get_bone_pose(posebone):
    if posebone.parent != None:
        parentRefPoseMtx = posebone.parent.bone.matrix_local
        boneRefPoseMtx = posebone.bone.matrix_local
        parentPoseMtx = posebone.parent.matrix
        bonePoseMtx = posebone.matrix
        boneLocMtx = (parentRefPoseMtx.inverted() @ boneRefPoseMtx).inverted() @ (parentPoseMtx.inverted() @ bonePoseMtx)
    else:
        boneRefPoseMtx = posebone.bone.matrix_local
        bonePoseMtx = posebone.matrix
        boneLocMtx = boneRefPoseMtx.inverted() @ bonePoseMtx

    return boneLocMtx.decompose()

def append_track_vec3(track, time, value, err):
    tracklen = len(track)
    if tracklen == 0 or (track[tracklen - 1][1] - value).length > err:
        track.append([time, value])

def append_track_quat(track, time, value, err):
    tracklen = len(track)
    if tracklen == 0:
        track.append([time, value])
    else:
        diff = track[tracklen - 1][1].rotation_difference(value)
        if degrees(diff.angle) > err:
            track.append([time, value])

def write_dbanim(self, context, filepath, bonemap, armature, track, resample_framerate, position_error, rotation_error, scale_error):
    self.report({'INFO'}, 'Writing animation file: ' + filepath)
    f = open(filepath, 'wb')

    # id: char[4]
    f.write(b'DBA\0')

    # ver: u32
    f.write(struct.pack('I', 1))

    # each animation track in a DBA file has a channel ID and a binding ID which can be used to determine what the track is used for
    # we'll store bone ID in the channel ID, and then store binding ID as follows:
    # position - 0
    # rotation - 1
    # scale - 2

    # this way, when animating at runtime, it becomes simple to just iterate the skeleton and ask for animation track (bone id, 0) to get position curves for a bone

    begin = 0
    end = 0

    # get frame range for animation
    for strip in track.strips:
        if strip.frame_start < begin:
            begin = strip.frame_start
        if strip.frame_end > end:
            end = strip.frame_end

    # get current frame
    cur_frame = bpy.context.scene.frame_current

    duration = (end - begin)
    framestep = bpy.context.scene.render.fps / float(resample_framerate)
    total_frames = int(duration / framestep) # for now we just sample animation as linear keyframes at 15FPS
    if total_frames == 0:
        total_frames = 1

    # gather loc/rot/scale track info
    pos_tracks = {}
    rot_tracks = {}
    scale_tracks = {}

    for posebone in armature.pose.bones:
        pos_tracks[posebone.name] = []
        rot_tracks[posebone.name] = []
        scale_tracks[posebone.name] = []

    for i in range(total_frames):
        frame = begin + int(i * framestep)
        time = float(frame) / bpy.context.scene.render.fps
        bpy.context.scene.frame_set(frame)
        bpy.context.view_layer.update()
        for posebone in armature.pose.bones:
            loc,rot,scale = get_bone_pose(posebone)
            append_track_vec3(pos_tracks[posebone.name], time, loc, position_error)
            append_track_quat(rot_tracks[posebone.name], time, rot, rotation_error)
            append_track_vec3(scale_tracks[posebone.name], time, scale, scale_error)

    # ensure last frame is exported for proper looping
    if begin + int((total_frames - 1) * framestep) < end:
        time = float(end) / bpy.context.scene.render.fps
        bpy.context.scene.frame_set(int(end))
        bpy.context.view_layer.update()
        for posebone in armature.pose.bones:
            loc,rot,scale = get_bone_pose(posebone)
            append_track_vec3(pos_tracks[posebone.name], time, loc, position_error)
            append_track_quat(rot_tracks[posebone.name], time, rot, rotation_error)
            append_track_vec3(scale_tracks[posebone.name], time, scale, scale_error)

    # export position tracks
    for bone_name in pos_tracks:
        boneid = bonemap[bone_name]
        postrack = pos_tracks[bone_name]
        # write new vec3 track
        f.write(b'VEC3')
        # chunk size (12 bytes + (16 * key count))
        f.write(struct.pack('I', 12 + (16 * len(postrack))))
        # channel id, binding id
        f.write(struct.pack('II', boneid, 0))
        # key count
        f.write(struct.pack('I', len(postrack)))
        # key data
        for key in postrack:
            f.write(struct.pack('ffff', key[0], key[1].x, key[1].y, key[1].z))

    # export rotation tracks
    for bone_name in rot_tracks:
        boneid = bonemap[bone_name]
        rottrack = rot_tracks[bone_name]
        # write new quat track
        f.write(b'QUAT')
        # chunk size (12 bytes + (20 * key count))
        f.write(struct.pack('I', 12 + (20 * len(rottrack))))
        # channel id, binding id
        f.write(struct.pack('II', boneid, 1))
        # key count
        f.write(struct.pack('I', len(rottrack)))
        # key data
        for key in rottrack:
            f.write(struct.pack('fffff', key[0], key[1].x, key[1].y, key[1].z, key[1].w))

    # export scale tracks
    for bone_name in scale_tracks:
        boneid = bonemap[bone_name]
        scaletrack = scale_tracks[bone_name]
        # write new vec3 track
        f.write(b'VEC3')
        # chunk size (12 bytes + (16 * key count))
        f.write(struct.pack('I', 12 + (16 * len(scaletrack))))
        # channel id, binding id
        f.write(struct.pack('II', boneid, 2))
        # key count
        f.write(struct.pack('I', len(scaletrack)))
        # key data
        for key in scaletrack:
            f.write(struct.pack('ffff', key[0], key[1].x, key[1].y, key[1].z))

    bpy.context.scene.frame_set(cur_frame)
    f.close()

def write_dbmesh(self, context, filepath, export_skinning, export_tracks, skip_muted_tracks, resample_framerate, position_error, rotation_error, scale_error):
    meshes = []
    armatures = []

    # gather all meshes & armatures in scene
    for ob in bpy.data.objects:
        if ob.type == 'MESH':
            meshes.append(ob)
            continue
        if ob.type == 'ARMATURE':
            armatures.append(ob)
            continue

    if len(armatures) > 1:
        self.report({'WARN'}, 'Only single armature supported, this may produce incorrect results')

    self.report({'INFO'}, 'Writing mesh file: ' + filepath)
    f = open(filepath, 'wb')

    # id: char[4]
    f.write(b'DBM\0')

    # ver: u32
    f.write(struct.pack('I', 1))

    bonemap = {}

    if export_skinning and len(armatures) > 0:
        armature = armatures[0]
        # sanity check: skeleton cannot have more than 256 bones
        if len(armature.data.bones) > 256:
            self.report({'ERROR'}, 'Armature has more than 256 bones - this is not supported! Armature will be skipped')
        else:
            # chunk id: char[4]
            f.write(b"SKEL")
            # each node is written as: { invbindmat: mat4x4, restpose: mat4x4, bone_idx: u8, child_count: u8 } - 130 bytes
            # chunk size: u32
            f.write(struct.pack('I', 130 * len(armature.data.bones)))
            boneid = 0
            for bone in armature.data.bones:
                # store in map of bone name -> bone palette id (so when we write vertex skinning info we can translate group index to actual bone index)
                bonemap[bone.name] = boneid
                # inv bind matrix: mat4x4
                invbindmat = (blender_to_db_mat() @ bone.matrix_local).inverted()
                for row in invbindmat:
                    f.write(struct.pack('ffff', row[0], row[1], row[2], row[3]))
                # rest pose: mat4x4
                restpose = blender_to_db_mat() @ bone.matrix_local
                if bone.parent != None:
                    restpose = (blender_to_db_mat() @ bone.parent.matrix_local).inverted() @ restpose
                for row in restpose:
                    f.write(struct.pack('ffff', row[0], row[1], row[2], row[3]))
                # bone index: u8
                f.write(struct.pack('B', boneid))
                # child count: u8
                f.write(struct.pack('B', len(bone.children)))
                boneid += 1

    if export_tracks and len(armatures) > 0:
        armature = armatures[0]
        if armature.animation_data:
            for track in armature.animation_data.nla_tracks:
                track.is_solo = False
            for track in armature.animation_data.nla_tracks:
                if track.mute and skip_muted_tracks:
                    continue
                if track.name == '[Action Stash]':
                    continue
                track.is_solo = True
                write_dbanim(self, context, filepath.removesuffix('.dbm') + '_' + track.name + '.dba', bonemap, armature, track, resample_framerate, position_error, rotation_error, scale_error)
                track.is_solo = False

    # for each mesh: emit MESH chunk
    for mesh in meshes:
        # triangulate mesh
        bm = bmesh.new()
        depsgraph = bpy.context.evaluated_depsgraph_get()
        bm.from_object(mesh, depsgraph)
        bmesh.ops.triangulate(bm, faces=bm.faces[:])
        uv_layer = bm.loops.layers.uv.active
        color_layer = bm.loops.layers.color.active
        deform_layer = bm.verts.layers.deform.active

        # chunk id: char[4]
        f.write(b'MESH')
        # chunk size: u32
        f.write(struct.pack('I', 32 + 40 + 42 + (len(bm.faces) * 3 * 20)))

        # mesh ID: char[32]
        f.write(struct.pack('32s', mesh.name.encode('utf-8')))

        # use armature: u8

        # translation: vec3
        f.write(struct.pack('3f', mesh.location[0], mesh.location[1], mesh.location[2]))

        # rotation: quaternion
        f.write(struct.pack('4f', mesh.rotation_quaternion[1], mesh.rotation_quaternion[2], mesh.rotation_quaternion[3], mesh.rotation_quaternion[0]))

        # scale
        f.write(struct.pack('3f', mesh.scale[0], mesh.scale[1], mesh.scale[2]))

        # material info: 42 bytes
        if len(mesh.material_slots) > 0:
            material = mesh.material_slots[0].material
            # material ID: char[32]
            f.write(struct.pack('32s', material.name.encode('utf-8')))
            # has texture: u8
            if material.node_tree:
                has_tex = 0
                for node in material.node_tree.nodes:
                    if node.type == 'TEX_IMAGE':
                        has_tex = 1
                        break

                f.write(struct.pack('B', has_tex))
            # blend enable: u8
            if material.blend_method == 'BLEND':
                f.write(struct.pack('B', 1))
            else:
                f.write(struct.pack('B', 0))
            # enable backface culling: u8
            if material.use_backface_culling:
                f.write(struct.pack('B', 1))
            else:
                f.write(struct.pack('B', 0))
            # diffuse color: rgba32
            f.write(struct.pack('BBBB', int(material.diffuse_color[0] * 255), int(material.diffuse_color[1] * 255), int(material.diffuse_color[2] * 255), int(material.diffuse_color[3] * 255)))
            # specular color: rgb24
            f.write(struct.pack('BBB', int(material.specular_color[0] * 255), int(material.specular_color[1] * 255), int(material.specular_color[2] * 255)))
            # roughness
            f.write(struct.pack('B', int(material.roughness * 255)))
        else:
            # material ID: char[32]
            f.write(struct.pack('32x'))
            # has texture: u8
            f.write(struct.pack('B', 0))
            # blend enable: u8
            f.write(struct.pack('B', 0))
            # enable backface culling: u8
            f.write(struct.pack('B', 1))
            # diffuse color: rgba32
            f.write(struct.pack('BBBB', 255, 255, 255, 255))
            # specular color: rgb24
            f.write(struct.pack('BBB', 0, 0, 0))
            # roughness
            f.write(struct.pack('B', 255))

        # triangle count: u16
        f.write(struct.pack('H', len(bm.faces)))

        # vertex array: [pos: half3, normal: half3, color: rgba32, texcoord: half2, boneweight: rg16, boneidx: rg16] (20 bytes per vertex)
        for face in bm.faces:
            for vert in face.loops:
                tc = mathutils.Vector((0.0, 0.0))
                if uv_layer != None:
                    tc = vert[uv_layer].uv
                n = face.normal
                if face.smooth:
                    n = vert.vert.normal
                p = vert.vert.co
                col = mathutils.Vector((1.0, 1.0, 1.0, 1.0))
                if color_layer != None:
                    col = vert[color_layer]
                bweight = [0, 0]
                bidx = [0, 0]
                if deform_layer != None:
                    # iterate bone weights and store up to two highest weights
                    for group_idx, weight in vert.vert[deform_layer].items():
                        # translate group index to actual bone index
                        group_name = mesh.vertex_groups[group_idx].name
                        bone_idx = bonemap[group_name]
                        for i in range(2):
                            if weight > bweight[i]:
                                bweight[i] = weight
                                bidx[i] = bone_idx
                                break
                # renormalize bone weights (in case >2 bones were assigned to this vertex, we re-normalize it to just the two)
                if bweight[0] > 0 and bweight[1] > 0:
                    bweight_sum = bweight[0] + bweight[1]
                    bweight[0] /= bweight_sum
                    bweight[1] /= bweight_sum
                # note: -Z is forward in DreamBox, but -Y is forward in Blender
                f.write(struct.pack('eeeeeeBBBBeeBBBB', -p.x, p.z, p.y, -n.x, n.z, n.y, int(col.x * 255), int(col.y * 255), int(col.z * 255), int(col.w * 255), tc.x, 1.0 - tc.y, int(bweight[0] * 255), int(bweight[1] * 255), bidx[0], bidx[1]))
        
        # release mesh
        bm.free()

    f.close()

    return {'FINISHED'}

# ExportHelper is a helper class, defines filename and
# invoke() function which calls the file selector.
from bpy_extras.io_utils import ExportHelper
from bpy.props import StringProperty, BoolProperty, EnumProperty
from bpy.types import Operator

class ExportDBMesh(Operator, ExportHelper):
    """Exports a mesh for use with the DreamBox SDK"""
    bl_idname = "export_dbmesh.dbmesh"  # important since its how bpy.ops.import_test.some_data is constructed
    bl_label = "Export DreamBox Mesh"

    # ExportHelper mixin class uses this
    filename_ext = ".dbm"

    filter_glob: StringProperty(
        default="*.dbm",
        options={'HIDDEN'},
        maxlen=255,  # Max internal buffer length, longer would be clamped.
    )

    # List of operator properties, the attributes will be assigned
    # to the class instance from the operator settings before calling.
    export_skinning: BoolProperty(
        name="Export Skinning",
        description="Export skeleton information",
        default=True,
    )

    export_tracks: BoolProperty(
        name="Export NLA Tracks",
        description="Export NLA tracks as separate .DBA animation files",
        default=True,
    )

    skip_muted_tracks: BoolProperty(
        name="Skip Muted NLA Tracks",
        description="Skip muted NLA tracks when exporting .DBA animation files",
        default=True,
    )

    resample_framerate: bpy.props.IntProperty(
        name="Resample Framerate",
        description="Resample animations at the given framerate for export",
        default=15,
        min=1,
    )

    position_error: bpy.props.FloatProperty(
        name="Position Error",
        description="Error threshold for skipping position keyframes for export",
        default=0.1,
        min=0.0,
    )

    rotation_error: bpy.props.FloatProperty(
        name="Rotation Error",
        description="Error threshold for skipping rotation keyframes for export",
        default=0.5,
        min=0.0,
    )

    scale_error: bpy.props.FloatProperty(
        name="Scale Error",
        description="Error threshold for skipping scale keyframes for export",
        default=0.1,
        min=0.0,
    )

    def execute(self, context):
        return write_dbmesh(self, context, self.filepath, self.export_skinning, self.export_tracks, self.skip_muted_tracks, self.resample_framerate, self.position_error, self.rotation_error, self.scale_error)

# Only needed if you want to add into a dynamic menu
def menu_func_export(self, context):
    self.layout.operator(ExportDBMesh.bl_idname, text="DBMesh (.dbm)")

# Register and add to the "file selector" menu (required to use F3 search "DBMesh (.dbm)" for quick access).
def register():
    bpy.utils.register_class(ExportDBMesh)
    bpy.types.TOPBAR_MT_file_export.append(menu_func_export)

def unregister():
    bpy.utils.unregister_class(ExportDBMesh)
    bpy.types.TOPBAR_MT_file_export.remove(menu_func_export)